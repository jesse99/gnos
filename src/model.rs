import to_str::to_str;
import rrdf::{create_store, get_blank_name, store, solution, solution_row_methods,
	triple, string_value, dateTime_value, selector, compile, solution_row_methods, solution_methods,
	store_methods};
import rrdf::solution::solution_row_trait;
import rrdf::store::{base_iter, store_trait};
import rrdf::store::to_str; 

export update_fn, msg, query_msg, update_msg, register_msg, deregister_msg, manage_state, get_state,
	alert, alert_level, error_level, warning_level, info_level, debug_level, open_alert, close_alert;

/// Function used to update a store within the model task.
///
/// Data can be anything, but is typically json. Return true if the store was updated.
type update_fn = fn~ (store: store, data: ~str) -> bool;

/// Enum used to communicate with the model task.
///
/// Used to query a model, to update a model, and to (un)register
/// server-sent events. First argument of all of constructors is the 
/// name of a store, e.g. "model" or "alerts".
enum msg
{
	query_msg(~str, ~str, comm::chan<solution>),			// SPARQL query + channel to send results back along
	update_msg(~str, update_fn, ~str),							// function to use to update the store + data to use
	
	register_msg(~str, ~str, ~str, comm::chan<solution>),	// key + SPARQL query + channel to send results back along
	deregister_msg(~str, ~str),									// key
}

/// Alerts are conditions that hold for a period of time (e.g. a router off line).
///
/// * device - is either an ip address or "server".
/// * id - is used along with device to identify alerts.
/// * level - is the severity of the alert.
/// * mesg - text that describes the alert (e.g. "offline").
/// * resolution - text that describes how to fix the alert (e.g. "Is the device on? Is the ip correct? Is it connected to the network?").
///
/// When an alert is added to the "alerts" store a gnos:begin dateTime is
/// included. When the alert becomes inactive (i.e. the condition no longer
/// holds) a gnos:end timestamp is added.
type alert = {device: ~str, id: ~str, level: alert_level, mesg: ~str, resolution: ~str};

enum alert_level
{
	error_level,
	warning_level,
	info_level,
	debug_level,
}

/// Runs within a task and manages triple stores holding gnos state.
///
/// Other tasks (e.g. views) can query or update the state this function manages.
fn manage_state(port: comm::port<msg>)
{
	let queries = std::map::str_hash();
	let mut listeners = std::map::str_hash();
	let namespaces = ~[
		{prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		{prefix: ~"snmp", path: ~"http://www.gnos.org/2012/snmp/"},
	];
	
	let stores = std::map::str_hash();
	stores.insert(~"model",  create_store(namespaces, @std::map::str_hash()));
	stores.insert(~"alerts",  create_store(namespaces, @std::map::str_hash()));
	
	listeners.insert(~"model", std::map::str_hash());
	listeners.insert(~"alerts", std::map::str_hash());
	
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
				if f(stores.get(name), data)
				{
					#info["Updated %s store", name];
					let updated = update_listeners(stores.get(name), queries, listeners[name]);
					listeners.insert(name, updated);
					if name == ~"alerts"
					{
						for iter::eachi(stores.get(name))
						|i, statement: triple|
						{
							#info["%?: %s", i, statement.to_str()];
						}
					}
				}
			}
			register_msg(name, key, query, channel)
			{
				let added = listeners[name].insert(key, (query, channel));
				assert added;
				
				send_solution(stores.get(name), queries, query, channel);
			}
			deregister_msg(name, key)
			{
				listeners[name].remove(key);
			}
		}
	}
}

/// Helper used to query model state.
fn get_state(name: ~str, channel: comm::chan<msg>, query: ~str) -> solution
{
	let port = comm::port::<solution>();
	let chan = comm::chan::<solution>(port);
	comm::send(channel, query_msg(name, query, chan));
	let result = comm::recv(port);
	ret result;
}

/// Helper used to add a new alert to the "alerts" store (if there is not already one open).
fn open_alert(store: store, alert: alert) -> bool
{
	let expr = #fmt["
	PREFIX gnos: <http://www.gnos.org/2012/schema#>
	SELECT
		?subject ?end
	WHERE
	{
		?subject gnos:device \"%s\" .
		?subject gnos:id \"%s\" .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
	}", alert.device, alert.id];
	
	alt eval_query(store, expr)
	{
		result::ok(solution)
		{
			// Add the alert if it doesn't already exist OR it exists but is closed.
			if solution.all(|row| {row.search(~"end").is_some()})
			{
				let level =
					alt alert.level
					{
						error_level		{~"error"}
						warning_level	{~"warning"}
						info_level			{~"info"}
						debug_level		{~"debug"}
					};
					
				let subject = get_blank_name(store, ~"alert");
				store.add(subject, ~[
					(~"gnos:device", string_value(alert.device, ~"")),
					(~"gnos:id", string_value(alert.id, ~"")),
					(~"gnos:begin", dateTime_value(std::time::now())),
					(~"gnos:mesg", string_value(alert.mesg, ~"")),
					(~"gnos:resolution", string_value(alert.resolution, ~"")),
					(~"gnos:level", string_value(level, ~"")),
				]);
				true
			}
			else
			{
				false
			}
		}
		result::err(err)
		{
			#error["open_alert> %s", err];
			false
		}
	}
}

/// Helper used to close any open alerts from the "alerts" store.
fn close_alert(store: store, device: ~str, id: ~str) -> bool
{
	let expr = #fmt["
	PREFIX gnos: <http://www.gnos.org/2012/schema#>
	SELECT
		?subject ?end
	WHERE
	{
		?subject gnos:device \"%s\" .
		?subject gnos:id \"%s\" .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
	}", device, id];
	
	alt eval_query(store, expr)
	{
		result::ok(solution)
		{
			let mut added = false;
			for solution.each
			|row|
			{
				if row.search(~"end").is_none()
				{
					store.add_triple(~[], {subject: row.get(~"subject").to_str(), predicate: ~"gnos:end", object: dateTime_value(std::time::now())});
					added = true;
				}
			}
			added
		}
		result::err(err)
		{
			#error["close_alert> %s", err];
			false
		}
	}
}

// ---- Internal functions ----------------------------------------------------

// In general the same queries will be used over and over again so it will be
// much more efficient to cache the selectors.
fn get_selector(queries: hashmap<~str, selector>, query: ~str) -> option::option<selector>
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

fn send_solution(store: store, queries: hashmap<~str, selector>, query: ~str, channel: comm::chan<solution>)
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

fn update_listeners(store: store, queries: hashmap<~str, selector>, listeners: hashmap<~str, (~str, comm::chan<solution>)>) -> hashmap<~str, (~str, comm::chan<solution>)>
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

fn eval_query(store: store, expr: ~str) -> result::result<solution, ~str>
{
	alt compile(expr)
	{
		result::ok(selector)
		{
			alt selector(store)
			{
				result::ok(solution)
				{
					result::ok(solution)
				}
				result::err(err)
				{
					result::err(#fmt["query failed to run: %s", err])
				}
			}
		}
		result::err(err)
		{
			result::err(#fmt["failed to compile query: expected %s", err])
		}
	}
}

