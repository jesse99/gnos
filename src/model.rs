//use to_str::to_str;
use std::map::*;
use rrdf::rrdf::*;
use Namespace = rrdf::solution::Namespace;

/// Function used to update a store within the model task.
///
/// Data can be anything, but is typically json. Return true if the store was updated.
type UpdateFn = fn~ (store: &Store, data: ~str) -> bool;

/// Like UpdateFn except that it takes multiple stores.
type UpdatesFn = fn~ (store: ~[@Store], data: ~str) -> bool;

/// The channel used by RegisterMsg to communicate the initial result and
/// subsequent results back to the original task.
///
/// In the case of an error only the initial result is sent.
type RegisterChan = comm::Chan<result::Result<~[Solution], ~str>>;

/// Enum used to communicate with the model task.
///
/// Used to query a model, to update a model, and to (un)register
/// server-sent events. Store should be "model" or "alerts".
enum Msg
{
	QueryMsg(~str, ~str, comm::Chan<Solution>),		// store + SPARQL query + channel to send results back along
	UpdateMsg(~str, UpdateFn, ~str),						// store + function to use to update the store + data to use
	UpdatesMsg(~[~str], UpdatesFn, ~str),				// stores + function to use to update the stores + data to use
	
	RegisterMsg(~str, ~str, ~[~str], RegisterChan),		// store + key + SPARQL queries + channel to send results back along
	DeregisterMsg(~str, ~str),								// store + key
	
	SyncMsg(comm::Chan<bool>),						// ensure the model task has processed all messages (for unit testing)
	ExitMsg,												// exits the task (for unit testing)
}

/// Alerts are conditions that hold for a period of time (e.g. a router off line).
///
/// * device - devices:<ip> or gnos:map.
/// * id - is used along with device to identify alerts.
/// * level - is the severity of the alert.
/// * mesg - text that describes the alert (e.g. "offline").
/// * resolution - text that describes how to fix the alert (e.g. "Is the device on? Is the ip correct? Is it connected to the network?").
///
/// When an alert is added to the "alerts" store a gnos:begin dateTime is
/// included. When the alert becomes inactive (i.e. the condition no longer
/// holds) a gnos:end timestamp is added.
struct Alert
{
	pub device: ~str,
	pub id: ~str,
	pub level: AlertLevel,
	pub mesg: ~str,
	pub resolution: ~str,
}

enum AlertLevel
{
	ErrorLevel,
	WarningLevel,
	InfoLevel,
	DebugLevel,
}

// TODO: This is hopefully temporary: at some point rust should again be able to compare enums without assistence.
impl AlertLevel : cmp::Eq
{
	pure fn eq(&&rhs: AlertLevel) -> bool
	{
		fmt!("%?", self) == fmt!("%?", rhs)
	}
	
	pure fn ne(&&rhs: AlertLevel) -> bool
	{
		fmt!("%?", self) != fmt!("%?", rhs)
	}
}

type Registration = {queries: ~[~str], channel: RegisterChan, solutions: @mut ~[Solution]};

pure fn get_standard_store_names() -> ~[~str]
{
	return ~[~"globals", ~"primary", ~"alerts", ~"snmp"];
}

/// Runs within a task and manages triple stores holding gnos state.
///
/// Other tasks (e.g. views) can query or update the state this function manages.
fn manage_state(port: comm::Port<Msg>)
{
	let namespaces = ~[
		Namespace {prefix: ~"devices", path: ~"http://network/"},
		Namespace {prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		Namespace {prefix: ~"snmp", path: ~"http://snmp/"},
		Namespace {prefix: ~"sname", path: ~"http://snmp-name/"},
	];
	
	let stores = std::map::HashMap();
	let queries = std::map::HashMap();		// query string => compiled query (cache)
	let registered = std::map::HashMap();		// store name => {registrar key => (query string, channel<Solution>)}
	
	for get_standard_store_names().each
	|name|
	{
		stores.insert(copy *name,  @Store(namespaces, &std::map::HashMap()));
		registered.insert(copy *name, std::map::HashMap());
	}
	
	loop
	{
		match comm::recv(port)
		{
			QueryMsg(name, expr, channel) =>
			{
				let solutions = eval_queries(stores.get(copy name), queries, ~[copy expr]).get();		// always a canned query so we want to fail fast on error
				assert solutions.len() == 1;
				comm::send(channel, copy solutions[0]);
			}
			UpdateMsg(name, f, data) =>
			{
				if f(stores.get(copy name), data)
				{
					info!("Updated %s store", name);
					update_registered(stores, name, queries, registered);
				}
			}
			UpdatesMsg(names, f, data) =>
			{
				// This is a bit of a lame special case, but there are some advantages:
				// 1) It allows multiple stores to be updated atomically.
				// 2) At the moment json is not sendable so we can use this message to avoid re-parsing
				// the (potentially very large) json strings modelers send us.
				let ss = do names.map |name| {stores.get(copy name)};
				if f(ss, data)
				{
					info!("Updated %s stores", str::connect(names, ~", "));
					for names.each
					|name|
					{
						update_registered(stores, *name, queries, registered);
					}
				}
			}
			RegisterMsg(name, key, exprs, channel) =>
			{
				match eval_queries(stores.get(copy name), queries, exprs)
				{
					result::Ok(solutions) =>
					{
						comm::send(channel, result::Ok(copy(solutions)));
						
						let added = registered[name].insert(copy key, {queries: copy exprs, channel: channel, solutions: @mut copy solutions});
						assert added;
					}
					result::Err(err) =>
					{
						comm::send(channel, result::Err(copy err));
					}
				}
			}
			DeregisterMsg(name, key) =>
			{
				registered[name].remove(copy key);
			}
			SyncMsg(channel) =>
			{
				comm::send(channel, true);
			}
			ExitMsg =>
			{
				break;
			}
		}
	}
}

/// Helper used to query model state.
fn get_state(name: ~str, channel: comm::Chan<Msg>, query: ~str) -> Solution
{
	let port = comm::Port::<Solution>();
	let chan = comm::Chan::<Solution>(port);
	comm::send(channel, QueryMsg(copy name, copy query, chan));
	let result = comm::recv(port);
	return result;
}

/// Helper used to add a new alert to the "alerts" store (if there is not already one open).
fn open_alert(store: &Store, alert: &Alert) -> bool
{
	let expr = #fmt["
	PREFIX devices: <http://network/>
	PREFIX gnos: <http://www.gnos.org/2012/schema#>
	SELECT
		?subject ?end
	WHERE
	{
		?subject gnos:target %s .
		?subject gnos:id \"%s\" .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
	}", alert.device, alert.id];
	
	match eval_query(store, expr)
	{
		result::Ok(solution) =>
		{
			// Add the alert if it doesn't already exist OR it exists but is closed (i.e. if we found rows they must all be closed).
			if solution.rows.all(|row| {row.search(~"end").is_some()})
			{
				let level =
					match alert.level
					{
						ErrorLevel =>		{~"error"}
						WarningLevel =>	{~"warning"}
						InfoLevel =>		{~"info"}
						DebugLevel =>	{~"debug"}
					};
					
				let subject = get_blank_name(store, ~"alert");
				store.add(subject, ~[
					(~"gnos:target", IriValue(copy alert.device)),
					(~"gnos:id", StringValue(copy alert.id, ~"")),
					(~"gnos:begin", DateTimeValue(std::time::now())),
					(~"gnos:mesg", StringValue(copy alert.mesg, ~"")),
					(~"gnos:resolution", StringValue(copy alert.resolution, ~"")),
					(~"gnos:level", StringValue(level, ~"")),
				]);
				
				if alert.level == ErrorLevel
				{
					update_err_count(store, alert.device, 1);
				}
				true
			}
			else
			{
				false
			}
		}
		result::Err(err) =>
		{
			error!("open_alert> %s", err);
			false
		}
	}
}

/// Helper used to close any open alerts from the "alerts" store.
fn close_alert(store: &Store, device: ~str, id: ~str) -> bool
{
	let expr = #fmt["
	PREFIX devices: <http://network/>
	PREFIX gnos: <http://www.gnos.org/2012/schema#>
	SELECT
		?subject ?level ?end
	WHERE
	{
		?subject gnos:target %s .
		?subject gnos:id \"%s\" .
		?subject gnos:level ?level .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
	}", device, id];
	
	match eval_query(store, expr)
	{
		result::Ok(solution) =>
		{
			let mut changed = false;
			let mut added = 0;
			for solution.rows.each
			|row|
			{
				if row.search(~"end").is_none()
				{
					if row.get(~"level").as_str() == ~"error"
					{
						added += 1;
					}
					store.add_triple(~[], {subject: row.get(~"subject").to_str(), predicate: ~"gnos:end", object: DateTimeValue(std::time::now())});
					changed = true;
				}
			}
			if added > 0
			{
				assert added == 1;
				update_err_count(store, device, -1);
			}
			changed
		}
		result::Err(err) =>
		{
			error!("close_alert> %s", err);
			false
		}
	}
}

fn eval_query(store: &Store, expr: ~str) -> result::Result<Solution, ~str>
{
	match compile(expr)
	{
		result::Ok(selector) =>
		{
			match selector(store)
			{
				result::Ok(solution) =>
				{
					result::Ok(copy solution)
				}
				result::Err(err) =>
				{
					result::Err(fmt!("query failed to run: %s", err))
				}
			}
		}
		result::Err(err) =>
		{
			result::Err(fmt!("failed to compile query: expected %s", err))
		}
	}
}
// ---- Internal functions ----------------------------------------------------
priv fn update_registered(stores: HashMap<~str, @Store>, name: ~str, queries: HashMap<~str, Selector>, registered: HashMap<~str, HashMap<~str, Registration>>)
{
	let store = stores.find(copy name);
	if store.is_some()
	{
		let map = registered.find(copy name);
		if option::is_some(map)
		{
			for map.get().each_value
			|r|
			{
				let solutions = eval_queries(store.get(), queries, r.queries).get();	// query that worked once so should be OK to fail fast
				if solutions != *r.solutions
				{
					comm::send(r.channel, result::Ok(copy(solutions)));
					*r.solutions = solutions;
				}
			}
		}
	}
}

priv fn update_err_count(store: &Store, device: ~str, delta: i64)
{
	match store.find_object(device, ~"gnos:num_errors")
	{
		option::Some(IntValue(value)) =>
		{
			// TODO: This is a rather inefficient pattern (though it doesn't matter here because
			// subject has only one predicate). But maybe replace_triple should have a variant 
			// or something that passes the original value to a closure.
			store.replace_triple(~[], {subject: copy device, predicate: ~"gnos:num_errors", object: IntValue(value + delta)});
		}
		option::Some(x) =>
		{
			fail fmt!("Expected an int value for gnos:num_errors in the alerts store, but found %?", x);
		}
		option::None =>
		{
			assert delta == 1;		// if we're closing an alert we should have found the err_count for the open alert
			store.add_triple(~[], {subject: copy device, predicate: ~"gnos:num_errors", object: IntValue(1)});
		}
	}
}

// In general the same queries will be used over and over again so it will be
// much more efficient to cache the selectors.
priv fn get_selector(queries: HashMap<~str, Selector>, query: ~str) -> result::Result<Selector, ~str>
{
	match queries.find(copy query)
	{
		option::Some(s) =>
		{
			result::Ok(s)
		}
		option::None =>
		{
			match compile(query)
			{
				result::Ok(s) =>
				{
					queries.insert(copy query, s);
					result::Ok(s)
				}
				result::Err(err) =>
				{
					error!("Failed to compile: expected %s", err);
					result::Err(copy err)
				}
			}
		}
	}
}

priv fn eval_queries(store: &Store, queries: HashMap<~str, Selector>, exprs: ~[~str]) -> result::Result<~[Solution], ~str>
{
	do result::map_vec(exprs)
	|expr|
	{
		match get_selector(queries, *expr)
		{
			result::Ok(selector) =>
			{
				match selector(store)
				{
					result::Ok(solution) =>
					{
						result::Ok(copy solution)
					}
					result::Err(err) =>
					{
						error!("'%s' failed with %s", *expr, err);
						result::Err(copy err)
					}
				}
			}
			result::Err(err) =>
			{
				result::Err(copy err)
			}
		}
	}
}

