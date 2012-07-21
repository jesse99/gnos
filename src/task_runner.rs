export failure_policy, ignore_failures, exit_on_failure, restart_on_failure, exit_fn,
	job_fn, job, run, sequence;

/// Actions to take if a job returns with an error code.
///
/// * ignore_failures - do nothing.
/// * exit_on_failure - call cleanup actions and then exit.
/// * restart_on_failure - call the job action again after waiting uint seconds, 
/// with uint max restarts, after calling f with the error message.
enum failure_policy
{
	ignore_failures,
	exit_on_failure,
	restart_on_failure(uint, uint, fn~ (str)),
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
		exit_on_failure
		{
			let err = job.action();
			if err.is_some()
			{
				#error["%s", err.get()];
				for cleanup.each |f| {f()};
				libc::exit(3);
			}
		}
		restart_on_failure(delay, max_retries, notify)
		{
			let mut count = 0;
			loop
			{
				alt job.action()
				{
					option::some(err)
					{
						notify(err);
						count += 1;
						if count <= max_retries
						{
							libc::funcs::posix88::unistd::sleep(delay as libc::c_uint);
						}
						else
						{
							break;
						}
					}
					option::none
					{
						break;
					}
				}
			}
		}
	}
}

