/// Functions and types used to manage a task responsible for managing RDF stores.
use std::map::{HashMap};
use std::time;
use oldcomm::{Chan, Port};
use std::json::ToJson;
use std::json::to_str;
use rrdf::*;
use Namespace = rrdf::solution::Namespace;

/// Function used to update a store within the model task.
///
/// Data can be anything, but is typically json. Return true if the store was updated.
pub type UpdateFn = fn~ (store: &Store, data: &str) -> bool;

/// Like UpdateFn except that it takes multiple stores.
pub type UpdatesFn = fn~ (store: &[@Store], data: &str) -> bool;

/// The channel used by RegisterMsg to communicate the initial result and
/// subsequent results back to the original task.
///
/// In the case of an error only the initial result is sent.
pub type RegisterChan = Chan<result::Result<std::json::Json, ~str>>;

/// Enum used to communicate with the model task.
///
/// Used to query a model, to update a model, and to (un)register
/// server-sent events. Store should be "model" or "alerts".
pub enum Msg
{
	QueryMsg(~str, ~str, Chan<std::json::Json>),		// store + SPARQL query + channel to send results back along (store prefixes are auto-added to the query)
	UpdateMsg(~str, UpdateFn, ~str),					// store + function to use to update the store + data to use
	UpdatesMsg(~[~str], UpdatesFn, ~str),			// stores + function to use to update the stores + data to use
	
	RegisterMsg(~str, ~str, ~[~str], RegisterChan),	// store + key + SPARQL queries + channel to send results back along
	DeregisterMsg(~str, ~str),							// store + key
	
	SyncMsg(Chan<bool>),							// ensure the model task has processed all messages (for unit testing)
	ExitMsg,											// exits the task (for unit testing)
}

/// Alerts are conditions that hold for a period of time (e.g. a router off line).
///
/// * target - map:auto-fat/entities/10.1.0.1 or gnos:container.
/// * id - is used along with target to identify alerts.
/// * mesg - text that describes the alert (e.g. "offline").
/// * resolution - text that describes how to fix the alert (e.g. "Is the device on? Is the ip correct? Is it connected to the network?").
/// * level - "error", "warning", or "info".
///
/// When an alert is added to the store a gnos:begin dateTime is
/// included. When the alert becomes inactive (i.e. the condition no longer
/// holds) a gnos:end timestamp is added.
pub struct Alert
{
	pub target: ~str,
	pub id: ~str,
	pub mesg: ~str,
	pub resolution: ~str,
	pub level: ~str,
}

pub type Registration = {queries: ~[~str], channel: RegisterChan, solutions: @mut ~[Solution]};

pub pure fn get_standard_store_names() -> ~[~str]
{
	return ~[~"globals", ~"primary"];
}

/// Runs within a task and manages triple stores holding gnos state.
///
/// Other tasks (e.g. views) can query or update the state this function manages.
pub fn manage_state(port: Port<Msg>, server: &str,  server_port: u16)
{
	// TODO: probably want to restart this task (and possibly others) on failure
	let namespaces = ~[
		Namespace {prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		Namespace {prefix: ~"map", path: fmt!("http://%s:%?/map/", server, server_port)},
	];
	
	let stores = HashMap();
	let queries = HashMap();			// query string => compiled query (cache)
	let registered = HashMap();		// store name => {registrar key => (query string, Chan<Solution>)}
	
	for get_standard_store_names().each
	|name|
	{
		// sparql prefixed names only support a single name after the colon so these are useful
		// (rrdf prefixed names support paths which gets a bit confusing)
		let namespaces = 
			~[
				Namespace {prefix: ~"entities", path: fmt!("http://%s:%?/map/%s/entities/", server, server_port, *name)},
				Namespace {prefix: ~"store", path: fmt!("http://%s:%?/map/%s/", server, server_port, *name)},
			] + namespaces;
		
		stores.insert(copy *name,  @Store(namespaces, &HashMap()));
		registered.insert(copy *name, HashMap());
	}
	
	loop
	{
		match oldcomm::recv(port)
		{
			QueryMsg(copy name, copy expr, channel) =>
			{
				let solutions = eval_queries(stores.get(name), queries, ~[expr]).get();		// always a canned query so we want to fail fast on error
				assert solutions.len() == 1;
				oldcomm::send(channel, solutions_to_json(solutions));
			}
			UpdateMsg(copy name, ref f, ref data) =>
			{
				if (*f)(stores.get(copy name), *data)
				{
					info!("Updated %s store", name);
					update_registered(stores, name, queries, registered);
				}
			}
			UpdatesMsg(copy names, ref f, ref data) =>
			{
				// This is a bit of a lame special case, but there are some advantages:
				// 1) It allows multiple stores to be updated atomically.
				// 2) At the moment json is not sendable so we can use this message to avoid re-parsing
				// the (potentially very large) json strings modelers send us.
				let ss = do names.map |name| {stores.get(name.to_owned())};
				if (*f)(ss, *data)
				{
					info!("Updated %s stores", str::connect(names, ~", "));
					for names.each
					|name|
					{
						update_registered(stores, *name, queries, registered);
					}
				}
			}
			RegisterMsg(copy name, copy key, copy exprs, channel) =>
			{
				// Normally queries should not fail, but users can construct custom queries
				// in the client which can be totally nutso so we need to be careful to avoid
				// failing.
				if stores.contains_key(copy name)
				{
					match eval_queries(stores.get(copy name), queries, exprs)
					{
						result::Ok(move solutions) =>
						{
							oldcomm::send(channel, result::Ok(solutions_to_json(solutions)));
							
							let added = registered[name].insert(key, {queries: exprs, channel: channel, solutions: @mut solutions});
							assert added;
						}
						result::Err(copy err) =>
						{
							oldcomm::send(channel, result::Err(~"Expected " + err));
						}
					}
				}
				else
				{
					oldcomm::send(channel, result::Err(fmt!("%s is not a valid store name", name)));
				}
			}
			DeregisterMsg(ref name, copy key) =>
			{
				if registered.contains_key(copy *name)
				{
					registered[copy *name].remove(key);
				}
			}
			SyncMsg(channel) =>
			{
				oldcomm::send(channel, true);
			}
			ExitMsg =>
			{
				break;
			}
		}
	}
}

/// Helper used to query model state.
pub fn get_state(name: &str, channel: Chan<Msg>, query: &str) -> std::json::Json
{
	let port = Port();
	let chan = Chan(&port);
	oldcomm::send(channel, QueryMsg(name.to_owned(), query.to_owned(), chan));
	let result = oldcomm::recv(port);
	return result;
}

priv fn get_prefixes(store: &Store) -> ~str
{
	let prefixes = do store.namespaces.filter_map |ns|
		{
			if ns.prefix != ~"_"
			{
				option::Some(fmt!("\tPREFIX %s: <%s>", ns.prefix, ns.path))
			}
			else
			{
				option::None
			}
		};
	str::connect(prefixes, "\n")
}

/// Helper used to add a new alert to a store (if there is not already one open).
pub fn open_alert(store: &Store, alert: &Alert) -> bool
{
	let expr = fmt!("
	%s
	SELECT
		?subject ?end
	WHERE
	{
		?subject gnos:target %s .
		?subject gnos:alert \"%s\" .
		OPTIONAL
		{
			?subject gnos:end ?end .
		}
	}", get_prefixes(store), alert.target, alert.id);
	
	match eval_query(store, expr)
	{
		result::Ok(ref solution) =>
		{
			// Add the alert if it doesn't already exist OR it exists but is closed (i.e. if we found rows they must all be closed).
			let i = solution.bindings.position_elem(&~"end");		// will be 1 (until the query changes anyway)
			if solution.rows.all(|row| {!row[i.get()].is_unbound()})
			{
				let subject = get_blank_name(store, ~"alert");
				store.add(subject, ~[
					(~"gnos:target", @IriValue(copy alert.target)),
					(~"gnos:alert", @StringValue(copy alert.id, ~"")),
					(~"gnos:begin", @DateTimeValue(std::time::now())),
					(~"gnos:mesg", @StringValue(copy alert.mesg, ~"")),
					(~"gnos:resolution", @StringValue(copy alert.resolution, ~"")),
					(~"gnos:style", @StringValue(~"alert-type:" + alert.level, ~"")),
				]);
				
				if alert.level == ~"error"
				{
					update_err_count(store, 1);
				}
				true
			}
			else
			{
				false
			}
		}
		result::Err(ref err) =>
		{
			error!("open_alert> %s", *err);
			error!("open_alert> %s", expr);
			false
		}
	}
}

/// Helper used to close any open alerts from the store.
pub fn close_alert(store: &Store, target: &str, id: &str) -> bool
{
	let expr = fmt!("
	%s
	SELECT
		?subject ?style ?end
	WHERE
	{
		?subject gnos:alert \"%s\" .
		?subject gnos:target %s .
		?subject gnos:style ?style .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
	}" , get_prefixes(store),id, target);
	
	match eval_query(store, expr)
	{
		result::Ok(ref solution) =>
		{
			let mut changed = false;
			let mut added = 0;
			let subject_index = solution.bindings.position_elem(&~"subject").get();	// will be 0 (until the query changes anyway)
			let style_index = solution.bindings.position_elem(&~"style").get();		// will be 1 (until the query changes anyway)
			let end_index = solution.bindings.position_elem(&~"end").get();		// will be 2 (until the query changes anyway)
			for solution.rows.each |row|
			{
				if row[end_index].is_unbound()
				{
					if row[style_index].as_str().ends_with(~":error")
					{
						added += 1;
					}
					store.add_triple(~[], {subject: (*row[subject_index]).to_str(), predicate: ~"gnos:end", object: @DateTimeValue(std::time::now())});
					changed = true;
				}
			}
			if added > 0
			{
				assert added == 1;
				update_err_count(store, -1);
			}
			if changed
			{
			}
			changed
		}
		result::Err(ref err) =>
		{
			error!("close_alert> %s", *err);
			error!("close_alert> %s", expr);
			false
		}
	}
}

pub fn eval_query(store: &Store, expr: &str) -> result::Result<Solution, ~str>
{
	match compile(expr)
	{
		result::Ok(selector) =>
		{
			let start = std::time::precise_time_s();
			match selector(store)
			{
				result::Ok(copy solution) =>
				{
					let elapsed = std::time::precise_time_s() - start;
					if elapsed > 0.333
					{
						//error!("evaluated %s", expr);
						error!("%? in %.3fs, %? rows", expr.len(), elapsed, solution.rows.len());
					}
					result::Ok(solution)
				}
				result::Err(ref err) =>
				{
					result::Err(fmt!("query failed to run: %s", *err))
				}
			}
		}
		result::Err(ref err) =>
		{
			result::Err(fmt!("failed to compile query: expected %s", *err))
		}
	}
}

// ---- Internal functions ----------------------------------------------------
priv fn update_registered(stores: HashMap<~str, @Store>, name: &str, queries: HashMap<~str, Selector>, registered: HashMap<~str, HashMap<~str, Registration>>)
{
	let store = stores.find(name.to_owned());
	if store.is_some()
	{
		let map = registered.find(name.to_owned());
		if option::is_some(&map)
		{
			for map.get().each_value
			|r|
			{
				let solutions = eval_queries(store.get(), queries, r.queries).get();	// query that worked once so should be OK to fail fast
				if solutions != *r.solutions
				{
					oldcomm::send(r.channel, result::Ok(solutions_to_json(solutions)));
					*r.solutions = solutions;
				}
			}
		}
	}
}

priv fn update_err_count(store: &Store, delta: i64)
{
	match store.find_object(~"store:globals", ~"gnos:num_errors")
	{
		option::Some(@IntValue(value)) =>
		{
			// TODO: This is a rather inefficient pattern (though it doesn't matter here because
			// subject has only one predicate). But maybe replace_triple should have a variant 
			// or something that passes the original value to a closure.
			store.replace_triple(~[], {subject: ~"store:globals", predicate: ~"gnos:num_errors", object: @IntValue(value + delta)});
		}
		option::Some(ref x) =>
		{
			fail fmt!("Expected an int value for gnos:num_errors in the alerts store, but found %?", x);
		}
		option::None =>
		{
			assert delta == 1;		// if we're closing an alert we should have found the err_count for the open alert
			store.add_triple(~[], {subject: ~"store:globals", predicate: ~"gnos:num_errors", object: @IntValue(1)});
		}
	}
}

// In general the same queries will be used over and over again so it will be
// much more efficient to cache the selectors.
priv fn get_selector(queries: HashMap<~str, Selector>, query: &str) -> result::Result<Selector, ~str>
{
	match queries.find(query.to_owned())
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
					queries.insert(query.to_owned(), s);
					result::Ok(s)
				}
				result::Err(copy err) =>
				{
					error!("Failed to compile: expected %s", err);
					error!("%s", query);
					result::Err(err)
				}
			}
		}
	}
}

priv fn eval_queries(store: &Store, queries: HashMap<~str, Selector>, exprs: &[~str]) -> result::Result<~[Solution], ~str>
{
	do result::map_vec(exprs)
	|expr|
	{
		let expr = get_prefixes(store) + *expr;
		match get_selector(queries, expr)
		{
			result::Ok(selector) =>
			{
				let start = std::time::precise_time_s();
				match selector(store)
				{
					result::Ok(copy solution) =>
					{
						let elapsed = std::time::precise_time_s() - start;
						if elapsed > 0.333
						{
							//error!("evaluated %s", expr);
							error!("%? in %.3fs, %? rows", expr.len(), elapsed, solution.rows.len());
						}
						result::Ok(solution)
					}
					result::Err(copy err) =>
					{
						error!("'%s' failed with %s", expr, err);
						result::Err(err)
					}
				}
			}
			result::Err(copy err) =>
			{
				result::Err(err)
			}
		}
	}
}

priv fn solutions_to_json(solutions: &[Solution]) -> std::json::Json
{
	if solutions.len() == 1
	{
		solution_to_json(&solutions[0])
	}
	else
	{
		std::json::List(
			do vec::map(solutions)
			|solution|
			{
				solution_to_json(solution)
			}
		)
	}
}

priv fn solution_to_json(solution: &Solution) -> std::json::Json
{
	//info!(" ");
	std::json::List(
		do vec::map(solution.rows) |row|
		{
			//info!("row: %?", row);
			solution_row_to_json(solution, row)
		}
	)
}

priv fn solution_row_to_json(solution: &Solution, row: &SolutionRow) -> std::json::Json
{
	let mut obj = ~send_map::linear::linear_map_with_capacity(row.len());
	
	for uint::range(0, solution.num_selected) |i|
	{
		if !row[i].is_unbound()
		{
			let value = object_to_json(row[i]);
			obj.insert(copy solution.bindings[i], value);
		}
	}
	
	std::json::Object(obj)
}

// TODO: need to escape as html? This should change to use auto-serialization.
priv fn object_to_json(obj: &Object) -> std::json::Json
{
	match *obj
	{
		IriValue(ref value) | BlankValue(ref value) =>
		{
			value.to_json()
		}
		UnboundValue(*) | InvalidValue(*) | ErrorValue(*) =>
		{
			// TODO: use custom css and render these in red
			obj.to_str().to_json()
		}
		StringValue(ref value, ~"") =>
		{
			value.to_json()
		}
		_ =>
		{
			obj.to_str().to_json()
		}
	}
}
