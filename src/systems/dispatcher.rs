use crate::storage::Ticks;
use crate::systems::{
    CommandBuffers, LocalFn, LocalSystem, Registry, RegistryAccess, RunError, RunResult, Runnable,
    System, SystemError,
};
use crate::world::{World, WorldId};
use rustc_hash::FxHashMap;
use std::mem;

#[cfg(feature = "parallel")]
use {
    rayon::iter::{IntoParallelRefMutIterator, ParallelIterator},
    rayon::ThreadPool,
};

/// Implements the builder pattern to create a `Dispatcher`.
#[derive(Default)]
pub struct DispatcherBuilder {
    simple_steps: Vec<SimpleStep>,
}

impl DispatcherBuilder {
    /// Add a system to the `Dispatcher`.
    pub fn add_system(&mut self, system: System) -> &mut Self {
        self.simple_steps.push(SimpleStep::RunSystem(system));
        self
    }

    /// Add a local system to the `Dispatcher` which runs on the current thread.
    pub fn add_local_system(&mut self, system: LocalSystem) -> &mut Self {
        self.simple_steps.push(SimpleStep::RunLocalSystem(system));
        self
    }

    /// Add a local system function to the `Dispatcher` which runs on the
    /// current thread.
    pub fn add_local_fn(&mut self, system: LocalFn) -> &mut Self {
        self.simple_steps.push(SimpleStep::RunLocalFn(system));
        self
    }

    /// Add a flush barrier which runs all the commands which need exclusive
    /// access to the `World` and `Resources`.
    pub fn add_flush(&mut self) -> &mut Self {
        self.simple_steps.push(SimpleStep::FlushCommands);
        self
    }

    /// Merge two `Dispatcher`s. After the call `other` is empty.
    pub fn merge(&mut self, other: &mut DispatcherBuilder) -> &mut Self {
        self.simple_steps.append(&mut other.simple_steps);
        self
    }

    /// Build a `Dispatcher` with the given systems and barriers.
    pub fn build(&mut self) -> Dispatcher {
        let steps = merge_and_optimize_steps(mem::take(&mut self.simple_steps));
        let command_buffers = CommandBuffers::new(required_command_buffers(&steps));

        Dispatcher { command_buffers, steps, change_ticks: Default::default() }
    }
}

/// Used to run `Systems`, potentially in parallel.
pub struct Dispatcher {
    steps: Vec<Step>,
    command_buffers: CommandBuffers,
    change_ticks: FxHashMap<WorldId, Ticks>,
}

impl Dispatcher {
    /// Creates a `DispatcherBuilder` to enable creating a `Dispatcher`
    /// using the builder pattern.
    pub fn builder() -> DispatcherBuilder {
        DispatcherBuilder::default()
    }

    /// Adds the required component storages to the `World` to avoid
    /// having to add them manually via `World::register`.
    pub fn register_storages(&self, world: &mut World) {
        for step in self.steps.iter() {
            match step {
                Step::RunSystems(systems) => {
                    for access in systems.iter().flat_map(|sys| sys.accesses()) {
                        match access {
                            RegistryAccess::Comp(comp) => unsafe {
                                world.register_with(comp.type_id(), || comp.create_storage());
                            },
                            RegistryAccess::CompMut(comp) => unsafe {
                                world.register_with(comp.type_id(), || comp.create_storage());
                            },
                            _ => (),
                        }
                    }
                }
                Step::RunLocalSystems(systems) => {
                    for access in systems.iter().flat_map(|sys| sys.accesses()) {
                        match access {
                            RegistryAccess::Comp(comp) => unsafe {
                                world.register_with(comp.type_id(), || comp.create_storage());
                            },
                            RegistryAccess::CompMut(comp) => unsafe {
                                world.register_with(comp.type_id(), || comp.create_storage());
                            },
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
    }

    /// Run all systems on the current thread.
    pub fn run_seq(&mut self, world: &mut World) -> RunResult {
        let world_tick = world.tick();
        let change_tick = self.change_ticks.entry(world.id()).or_default();

        let mut errors = Vec::<SystemError>::new();

        for step in self.steps.iter_mut() {
            match step {
                Step::RunSystems(systems) => {
                    run_systems_seq(
                        systems,
                        world,
                        &self.command_buffers,
                        *change_tick,
                        &mut errors,
                    );
                }
                Step::RunLocalSystems(systems) => {
                    run_systems_seq(
                        systems,
                        world,
                        &self.command_buffers,
                        *change_tick,
                        &mut errors,
                    );
                }
                Step::RunLocalFns(systems) => {
                    run_local_fns(systems, world, &mut errors);
                }
                Step::FlushCommands => {
                    world.maintain();

                    self.command_buffers.drain().for_each(|command| command(world));
                }
            }
        }

        *change_tick = world_tick;

        if !errors.is_empty() {
            Err(RunError::from(errors))
        } else {
            Ok(())
        }
    }

    /// Run all systems, potentially in parallel, on the given `ThreadPool`.
    #[cfg(feature = "parallel")]
    pub fn run_par(&mut self, world: &mut World, thread_pool: &ThreadPool) -> RunResult {
        let world_tick = world.tick();
        let change_tick = self.change_ticks.entry(world.id()).or_default();

        let mut errors = Vec::<SystemError>::new();

        for step in self.steps.iter_mut() {
            match step {
                Step::RunSystems(systems) => {
                    if systems.len() > 1 {
                        run_systems_par(
                            systems,
                            world,
                            &self.command_buffers,
                            *change_tick,
                            thread_pool,
                            &mut errors,
                        );
                    } else {
                        run_systems_seq(
                            systems,
                            world,
                            &self.command_buffers,
                            *change_tick,
                            &mut errors,
                        );
                    }
                }
                Step::RunLocalSystems(systems) => {
                    run_systems_seq(
                        systems,
                        world,
                        &self.command_buffers,
                        *change_tick,
                        &mut errors,
                    );
                }
                Step::RunLocalFns(systems) => {
                    run_local_fns(systems, world, &mut errors);
                }
                Step::FlushCommands => {
                    world.maintain();

                    self.command_buffers.drain().for_each(|command| command(world));
                }
            }
        }

        *change_tick = world_tick;

        if !errors.is_empty() {
            Err(RunError::from(errors))
        } else {
            Ok(())
        }
    }

    /// Returns the maximum number of systems that can run concurrently.
    /// Can be used to set the number of threads in the `rayon::ThreadPool`.
    pub fn max_concurrecy(&self) -> usize {
        let mut max_concurrecy = 1;

        for step in self.steps.iter() {
            if let Step::RunSystems(systems) = step {
                max_concurrecy = max_concurrecy.max(systems.len());
            }
        }

        max_concurrecy
    }
}

enum SimpleStep {
    RunSystem(System),
    RunLocalSystem(LocalSystem),
    RunLocalFn(LocalFn),
    FlushCommands,
}

enum Step {
    RunSystems(Vec<System>),
    RunLocalSystems(Vec<LocalSystem>),
    RunLocalFns(Vec<LocalFn>),
    FlushCommands,
}

fn count_command_buffers<R>(runnables: &[R]) -> usize
where
    R: Runnable,
{
    runnables
        .iter()
        .flat_map(R::accesses)
        .map(|access| matches!(access, RegistryAccess::Commands) as usize)
        .sum()
}

fn required_command_buffers(steps: &[Step]) -> usize {
    let mut max_buffer_count = 0;
    let mut buffer_count = 0;

    for step in steps {
        match step {
            Step::RunSystems(systems) => {
                buffer_count += count_command_buffers(systems);
            }
            Step::RunLocalSystems(systems) => {
                buffer_count += count_command_buffers(systems);
            }
            Step::FlushCommands => {
                max_buffer_count = max_buffer_count.max(buffer_count);
                buffer_count = 0;
            }
            _ => (),
        }
    }

    max_buffer_count
}

fn merge_and_optimize_steps(mut simple_steps: Vec<SimpleStep>) -> Vec<Step> {
    let mut steps = Vec::<Step>::new();

    for simple_step in simple_steps.drain(..).chain(Some(SimpleStep::FlushCommands)) {
        match simple_step {
            SimpleStep::RunSystem(system) => match steps.last_mut() {
                Some(Step::RunSystems(systems)) => {
                    let systems_conflict =
                        systems.iter().flat_map(System::accesses).any(|access1| {
                            system.accesses().iter().any(|access2| access1.conflicts(access2))
                        });

                    if systems_conflict {
                        steps.push(Step::RunSystems(vec![system]));
                    } else {
                        systems.push(system);
                    }
                }
                _ => {
                    steps.push(Step::RunSystems(vec![system]));
                }
            },
            SimpleStep::RunLocalSystem(system) => match steps.last_mut() {
                Some(Step::RunLocalSystems(systems)) => {
                    systems.push(system);
                }
                _ => steps.push(Step::RunLocalSystems(vec![system])),
            },
            SimpleStep::RunLocalFn(system) => match steps.last_mut() {
                Some(Step::RunLocalFns(systems)) => {
                    systems.push(system);
                }
                _ => steps.push(Step::RunLocalFns(vec![system])),
            },
            SimpleStep::FlushCommands => match steps.last() {
                Some(Step::FlushCommands) | None => (),
                _ => steps.push(Step::FlushCommands),
            },
        }
    }

    steps
}

fn run_systems_seq<S>(
    systems: &mut [S],
    world: &World,
    command_buffers: &CommandBuffers,
    change_tick: Ticks,
    errors: &mut Vec<SystemError>,
) where
    S: Runnable,
{
    let registry = unsafe { Registry::new(world, command_buffers, change_tick) };
    let new_errors = systems.iter_mut().flat_map(|sys| sys.run(&registry).err());
    errors.extend(new_errors);
}

#[cfg(feature = "parallel")]
fn run_systems_par(
    systems: &mut [System],
    world: &World,
    command_buffers: &CommandBuffers,
    change_tick: Ticks,
    thread_pool: &ThreadPool,
    errors: &mut Vec<SystemError>,
) {
    let registry = unsafe { Registry::new(world, command_buffers, change_tick) };

    thread_pool.install(|| {
        let new_errors = systems
            .par_iter_mut()
            .flat_map_iter(|sys| sys.run(&registry).err())
            .collect::<Vec<_>>();

        errors.extend(new_errors);
    });
}

fn run_local_fns(systems: &mut [LocalFn], world: &mut World, errors: &mut Vec<SystemError>) {
    let new_errors = systems.iter_mut().flat_map(|sys| sys.run(world).err());
    errors.extend(new_errors);
}
