// TODO: Should be able to simply import rrdf. See https://github.com/mozilla/rust/issues/1935
import rrdf::object::*;
import rrdf::store::*;

// Data can be anything, but is typically json.
type store_setter = fn~ (store: store, data: str) -> ();

enum msg
{
	getter(comm::chan<[triple]/~>),		// TODO: getter should take a (SPARQL query, chan<solution>)
	setter(store_setter, str)
}

// The manage_state function runs within a dedicated task and allows
// other tasks to get a snapshot of the model or update the model.
fn manage_state(port: comm::port<msg>)
{
	let store = create_store(
		[
			{prefix: "gnos", path: "http://www.gnos.org/2012/schema#"},
			{prefix: "snmp", path: "http://www.gnos.org/2012/snmp/"},
		]/~, []/~);
	
	loop
	{
		alt comm::recv(port)
		{
			getter(channel)
			{
				comm::send(channel, iter::to_vec(store));
			}
			setter(f, data)
			{
				f(store, data);
				//#info["Store set to:"];
				//for store.each
				//{|triple|
				//	#info["%s", triple.to_str()];
				//};
			}
		}
	}
}

fn get_state(channel: comm::chan<msg>) -> [triple]/~
{
	let port = comm::port::<[triple]/~>();
	let chan = comm::chan::<[triple]/~>(port);
	comm::send(channel, getter(chan));
	ret comm::recv(port);
}
