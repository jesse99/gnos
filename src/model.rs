// The manage_state function runs within a dedicated task and allows
// other tasks to get a snapshot of the model or update the model.
enum msg
{
	getter(comm::chan<[triple]>),
	setter([triple])
}

fn manage_state(port: comm::port<msg>)
{
	let mut state = [];
	
	loop
	{
		alt comm::recv(port)
		{
			getter(channel)
			{
				comm::send(channel, copy(state));
			}
			setter(new_state)
			{
				state = new_state;
			}
		}
	}
}

fn get_state(channel: comm::chan<msg>) -> [triple]
{
	let port = comm::port::<[triple]>();
	let chan = comm::chan::<[triple]>(port);
	comm::send(channel, getter(chan));
	ret comm::recv(port);
}
