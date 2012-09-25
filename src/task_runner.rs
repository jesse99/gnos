/// Actions to take after a job finishes.
///
/// * IgnoreFailures - do nothing.
/// * NotifyOnFailure - call a function with the error.
/// * NotifyOnExit - call a function with the error or option::none.
/// * ShutdownOnFailure - call cleanup actions and then call exit.
enum FailurePolicy
{
	IgnoreFailures,
	NotifyOnFailure(fn~ (~str)),
	NotifyOnExit(fn~ (option::Option<~str>)),
	ShutdownOnFailure,
}

/// A pointer to a function to call when the server shuts down.
type ExitFn = fn~ () -> ();

/// A pointer to a function to execute within a task.
///
/// Returns a message on errors.
type JobFn = fn~ () -> option::Option<~str>;

struct Job
{
	pub action: JobFn,
	pub policy: FailurePolicy,
}

/// Run the job within a task.
fn run(+job: Job, +cleanup: ~[ExitFn])
{
	// These guys can block for arbitrary amounts of time so they need their own thread.
	do task::spawn_sched(task::SingleThreaded)
	{
		do_run(&job, cleanup);
	}
}

/// Run the jobs within a task: one after another.
fn sequence(+jobs: ~[Job], +cleanup: ~[ExitFn])
{
	do task::spawn_sched(task::SingleThreaded)
	{
		for jobs.each
		|job|
		{
			do_run(job, cleanup);
		}
	}
}

priv fn do_run(job: &Job, cleanup: ~[ExitFn])
{
	match job.policy
	{
		IgnoreFailures =>
		{
			let err = job.action();
			if err.is_some()
			{
				info!("%s", err.get());
			}
		}
		NotifyOnFailure(ref notify) =>
		{
			let err = job.action();
			if err.is_some()
			{
				(*notify)(err.get());
			}
		}
		NotifyOnExit(ref notify) =>
		{
			(*notify)(job.action())
		}
		ShutdownOnFailure =>
		{
			let err = job.action();
			if err.is_some()
			{
				error!("%s", err.get());
				for cleanup.each |f| {(*f)()};
				libc::exit(3);
			}
		}
	}
}

