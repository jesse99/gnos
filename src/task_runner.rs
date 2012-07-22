export failure_policy, ignore_failures, notify_on_failure, notify_on_exit,
	shutdown_on_failure, exit_fn, job_fn, job, run, sequence;

/// Actions to take after a job finishes.
///
/// * ignore_failures - do nothing.
/// * notify_on_failure - call a function with the error.
/// * notify_on_exit - call a function with the error or option::none.
/// * shutdown_on_failure - call cleanup actions and then call exit.
enum failure_policy
{
	ignore_failures,
	notify_on_failure(fn~ (str)),
	notify_on_exit(fn~ (option::option<str>)),
	shutdown_on_failure,
}

/// A pointer to a function to call when the server shuts down.
type exit_fn = fn~ () -> ();

/// A pointer to a function to execute within a task.
///
/// Returns a message on errors.
type job_fn = fn~ () -> option::option<str>;

type job = {action: job_fn, policy: failure_policy};

/// Run the job within a task.
fn run(+job: job, +cleanup: ~[exit_fn])
{
	do task::spawn
	{
		do_run(job, cleanup);
	}
}

/// Run the jobs within a task: one after another.
fn sequence(+jobs: ~[job], +cleanup: ~[exit_fn])
{
	do task::spawn
	{
		for jobs.each
		|job|
		{
			do_run(job, cleanup);
		}
	}
}

fn do_run(job: job, cleanup: ~[exit_fn])
{
	alt job.policy
	{
		ignore_failures
		{
			let err = job.action();
			if err.is_some()
			{
				#info["%s", err.get()];
			}
		}
		notify_on_failure(notify)
		{
			let err = job.action();
			if err.is_some()
			{
				notify(err.get());
			}
		}
		notify_on_exit(notify)
		{
			notify(job.action())
		}
		shutdown_on_failure
		{
			let err = job.action();
			if err.is_some()
			{
				#error["%s", err.get()];
				for cleanup.each |f| {f()};
				libc::exit(3);
			}
		}
	}
}

