// Uses Server Sent Events to send solutions for a query after the model is updated.
use std::json::ToJson;
use std::json::to_str;
use model::{Msg, DeregisterMsg, RegisterMsg};
use rrdf::rrdf::*;
use server = rwebserve::rwebserve;
use rwebserve::imap::ImmutableMap;

export get_query;

/// Used by client code to register server-sent events for SPARQL queries.
///
/// The client EventSource is called when the sse is first registered and
/// again when the solution(s) returned by the query change. Two
/// different forms of the query string are supported:
///
/// * **name=foo&expr=blah** Name should be the name of a store.
/// Expr should be a SPARQL query. Result will be a solution encoded
/// as JSON: the solution is represented by a list and solution_rows
/// by dictionaries, e.g. [{"name": "bob", "age": 10"}, ...]. 
///
/// * **name=foo&expr=blah&expr2=blah&...** Like the above except
/// multiple queries can be run against the store. Result will be a list
/// of JSON encoded solutions.
///
/// If a query fails to compile the result will be a string with an error message.
fn get_query(state_chan: comm::Chan<Msg>, request: &server::Request, push: server::PushChan) -> server::ControlChan
{
	let name = *request.params.get(@~"name");
	let queries = get_queries(request);
	
	do utils::spawn_moded_listener(task::ManualThreads(2))
	|control_port: server::ControlPort|
	{
		info!("starting %s query stream", name);
		let notify_port = comm::Port();
		let notify_chan = comm::Chan(notify_port);
		
		let key = fmt!("query %?", ptr::addr_of(notify_port));
		comm::send(state_chan, RegisterMsg(name, key, queries, notify_chan));
		
		let mut solutions = ~[];
		loop
		{
			match comm::select2(notify_port, control_port)
			{
				either::Left(result::Ok(new_solutions)) =>
				{
					if new_solutions != solutions
					{
						solutions = new_solutions;	// TODO: need to escape the json?
						comm::send(push, fmt!("retry: 5000\ndata: %s\n\n", solutions_to_json(solutions).to_str()));
					}
					else
					{
					}
				}
				either::Left(result::Err(err)) =>
				{
					comm::send(push, fmt!("retry: 5000\ndata: %s\n\n", (~"Expected " + err).to_json().to_str()));
				}
				either::Right(server::RefreshEvent) =>
				{
					comm::send(push, fmt!("retry: 5000\ndata: %s\n\n", solutions_to_json(solutions).to_str()));
				}
				either::Right(server::CloseEvent) =>
				{
					info!("shutting down query stream");
					comm::send(state_chan, DeregisterMsg(name, key));
					break;
				}
			}
		}
	}
}

fn get_queries(request: &server::Request) -> ~[~str]
{
	let mut queries = ~[];
	vec::push(queries, *request.params.get(@~"expr"));
	
	for uint::iterate(2, 10)
	|i|
	{
		match request.params.find(@fmt!("expr%?", i))
		{
			option::Some(expr) =>
			{
				vec::push(queries, *expr);
			}
			option::None =>
			{
			}
		}
	};
	
	return queries;
}

fn solutions_to_json(solutions: &[Solution]) -> std::json::Json
{
	if solutions.len() == 1
	{
		solution_to_json(&solutions[0])
	}
	else
	{
		std::json::List(@
			do vec::map(solutions)
			|solution|
			{
				solution_to_json(&solution)
			}
		)
	}
}

fn solution_to_json(solution: &Solution) -> std::json::Json
{
	//info!(" ");
	std::json::List(@
		do vec::map(solution.rows)
		|row|
		{
			//info!("row: %?", row);
			solution_row_to_json(&row)
		}
	)
}

fn solution_row_to_json(row: &SolutionRow) -> std::json::Json
{
	std::json::Dict(
		std::map::hash_from_strs(
			do vec::map(*row)
			|entry|
			{
				let (key, value) = entry;
				(key, object_to_json(value))
			}
		)
	)
}

// TODO: need to escape as html?
fn object_to_json(obj: Object) -> std::json::Json
{
	match obj
	{
		IriValue(value) | BlankValue(value) =>
		{
			value.to_json()
		}
		UnboundValue(*) | InvalidValue(*) | ErrorValue(*) =>
		{
			// TODO: use custom css and render these in red
			obj.to_str().to_json()
		}
		StringValue(value, ~"") =>
		{
			value.to_json()
		}
		_ =>
		{
			obj.to_str().to_json()
		}
	}
}
