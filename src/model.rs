import rrdf::*;

export update_fn, msg, query_msg, update_msg, register_msg, deregister_msg, manage_state, get_state;

/// Function used to update a store within the model task.
///
/// Data can be anything, but is typically json.
type update_fn = fn~ (store: store, data: str) -> ();

/// Enum used to communicate with the model task.
///
/// Used to query a model, to update a model, and to (un)register
/// server-sent events. First argument of all of constructors (except
/// deregister_msg) is the name of a store, e.g. "model" or "alerts".
enum msg
{
	query_msg(str, str, comm::chan<solution>),			// SPARQL query + channel to send results back along
	update_msg(str, update_fn, str),							// function to use to update the store + data to use
	
	register_msg(str, str, str, comm::chan<solution>),	// key + SPARQL query + channel to send results back along
	deregister_msg(str),										// key
}

/// Runs within a task and manages triple stores holding gnos state.
///
/// Other tasks (e.g. views) can query or update the state this function manages.
fn manage_state(port: comm::port<msg>)
{
	let queries = std::map::str_hash();
	let mut listeners = std::map::str_hash();
	let namespaces = ~[
		{prefix: "gnos", path: "http://www.gnos.org/2012/schema#"},
		{prefix: "snmp", path: "http://www.gnos.org/2012/snmp/"},
	];
	
	let stores = std::map::str_hash();
	stores.insert("model",  create_store(namespaces, @std::map::str_hash()));
	stores.insert("alerts",  create_store(namespaces, @std::map::str_hash()));
	
	loop
	{
		alt comm::recv(port)
		{
			query_msg(name, query, channel)
			{
				send_solution(stores.get(name), queries, query, channel);
			}
			update_msg(name, f, data)
			{
				f(stores.get(name), data);
				#info["Updated store"];
				listeners = update_listeners(stores.get(name), queries, listeners);
			}
			register_msg(name, key, query, channel)
			{
				let added = listeners.insert(key, (query, channel));
				assert added;
				
				send_solution(stores.get(name), queries, query, channel);
			}
			deregister_msg(key)
			{
				listeners.remove(key);
			}
		}
	}
}

/// Helper used to query model state.
fn get_state(name: str, channel: comm::chan<msg>, query: str) -> solution
{
	let port = comm::port::<solution>();
	let chan = comm::chan::<solution>(port);
	comm::send(channel, query_msg(name, query, chan));
	let result = comm::recv(port);
	ret result;
}

// ---- Internal functions ----------------------------------------------------

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

