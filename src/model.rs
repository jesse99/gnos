import rrdf::*;

// Data can be anything, but is typically json.
type update_fn = fn~ (store: store, data: str) -> ();

enum msg
{
	query_msg(str, comm::chan<solution>),		// SPARQL query + channel to send results back along
	update_msg(update_fn, str),						// function to use to update the store + data to use
	
	register_msg(str, str, comm::chan<solution>),	// key + SPARQL query + channel to send results back along
	deregister_msg(str),								// key
}

// In general the same queries will be used over and over again so it will be
// much more efficient to cache the selectors.
fn get_selector(queries: hashmap<str, selector>, query: str) -> option::option<selector>
{
	alt queries.find(query)
	{
		option::some(s)
		{
			option::some(s)
		}
		option::none
		{
			alt compile(query)
			{
				result::ok(s)
				{
					queries.insert(query, s);
					option::some(s)
				}
				result::err(err)
				{
					#error["Failed to compile: expected %s", err];
					option::none
				}
			}
		}
	}
}

fn send_solution(store: store, queries: hashmap<str, selector>, query: str, channel: comm::chan<solution>)
{
	let selector = get_selector(queries, query);
	if option::is_some(selector)
	{
		alt option::get(selector)(store)
		{
			result::ok(solution)
			{
				comm::send(channel, copy(solution));
			}
			result::err(err)
			{
				#error["'%s' failed with %s", query, err];
				comm::send(channel, ~[]);
			}
		}
	}
	else
	{
		comm::send(channel, ~[]);
	}
}

fn update_listeners(store: store, queries: hashmap<str, selector>, listeners: hashmap<str, (str, comm::chan<solution>)>) -> hashmap<str, (str, comm::chan<solution>)>
{
	let new_listeners = std::map::str_hash();
	
	for listeners.each
	|key, value|
	{
		let (query, channel) = value;
		let selector = get_selector(queries, query);
		if option::is_some(selector)
		{
			alt option::get(selector)(store)
			{
				result::ok(solution)
				{
					comm::send(channel, copy(solution));
					new_listeners.insert(key, (query, channel));
				}
				result::err(err)
				{
					#error["'%s' failed with %s", query, err];
					new_listeners.remove(key);
				}
			}
		}
		else
		{
			new_listeners.remove(key);
		}
	};
	
	ret new_listeners;
}

// The manage_state function runs within a dedicated task and allows
// other tasks to get a snapshot of the model or update the model.
fn manage_state(port: comm::port<msg>)
{
	let queries = std::map::str_hash();
	let mut listeners = std::map::str_hash();
	let store = create_store(
		~[
			{prefix: "gnos", path: "http://www.gnos.org/2012/schema#"},
			{prefix: "snmp", path: "http://www.gnos.org/2012/snmp/"},
		], ~[]);
	
	loop
	{
		alt comm::recv(port)
		{
			query_msg(query, channel)
			{
				send_solution(store, queries, query, channel);
			}
			update_msg(f, data)
			{
				f(store, data);
				#info["Updated store"];
				listeners = update_listeners(store, queries, listeners);
			}
			register_msg(key, query, channel)
			{
				let added = listeners.insert(key, (query, channel));
				assert added;
				
				send_solution(store, queries, query, channel);
			}
			deregister_msg(key)
			{
				listeners.remove(key);
			}
		}
	}
}

fn get_state(channel: comm::chan<msg>, query: str) -> solution
{
	let port = comm::port::<solution>();
	let chan = comm::chan::<solution>(port);
	comm::send(channel, query_msg(query, chan));
	let result = comm::recv(port);
	ret result;
}
