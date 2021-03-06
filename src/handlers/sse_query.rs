/// Uses Server Sent Events to send solutions for a query after the model is updated.
use core::path::{GenericPath};
use oldcomm::{Chan, Port};
use model::{Msg, DeregisterMsg, RegisterMsg};
use rrdf::*;
use std::json::ToJson;
use server = rwebserve;
use Config = rwebserve::Config;
use Request = rwebserve::Request;
use Response = rwebserve::Response;
use ResponseHandler = rwebserve::ResponseHandler;

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
pub fn sse_query(state_chan: Chan<Msg>, request: &server::Request, push: server::PushChan) -> server::ControlChan
{
	let name = copy request.params.get(@~"name");
	let queries = get_queries(request);
	
	do utils::spawn_moded_listener(task::ThreadPerCore) |control_port: server::ControlPort|
	{
		info!("starting %s query stream", name);
		let notify_port = Port();
		let notify_chan = Chan(&notify_port);
		
		let key = fmt!("query %?", ptr::addr_of(&notify_port));
		oldcomm::send(state_chan, RegisterMsg(copy name, copy key, copy queries, notify_chan));
		
		let mut solutions = std::json::Null;
		loop
		{
			match oldcomm::select2(notify_port, control_port)
			{
				either::Left(result::Ok(ref new_solutions)) =>
				{
					if *new_solutions != solutions
					{
						solutions = copy *new_solutions;	// TODO: need to escape the json?
						oldcomm::send(push, fmt!("retry: 5000\ndata: %s\n\n", solutions.to_str()));
					}
				}
				either::Left(result::Err(ref err)) =>
				{
					oldcomm::send(push, fmt!("retry: 5000\ndata: %s\n\n", err.to_json().to_str()));
				}
				either::Right(server::RefreshEvent) =>
				{
					oldcomm::send(push, fmt!("retry: 5000\ndata: %s\n\n", solutions.to_str()));
				}
				either::Right(server::CloseEvent) =>
				{
					info!("shutting down query stream");
					oldcomm::send(state_chan, DeregisterMsg(copy name, key));
					break;
				}
			}
		}
	}
}

priv fn get_queries(request: &server::Request) -> ~[~str]
{
	let mut queries = ~[];
	vec::push(&mut queries, copy request.params.get(@~"expr"));
	
	for uint::iterate(2, 10) |i|
	{
		match request.params.find(@fmt!("expr%?", i))
		{
			option::Some(expr) =>
			{
				vec::push(&mut queries, copy expr);
			}
			option::None =>
			{
			}
		}
	};
	
	return queries;
}
