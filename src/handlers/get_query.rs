// Uses Server Sent Events to send solutions for a query after the model is updated.
import std::json::to_json;
import std::json::to_str;
import model::{msg, deregister_msg, register_msg};
import rrdf::{object, iri_value, blank_value, unbound_value, invalid_value, error_value, string_value,
	solution_row, solution};
import rwebserve::imap::{imap_methods, immutable_map};

export get_query;

// server-sent event handler
fn get_query(state_chan: comm::chan<msg>, request: server::request, push: server::push_chan) -> server::control_chan
{
	let name = request.params.get(~"name");
	let query = request.params.get(~"expr");
	
	do task::spawn_listener
	|control_port: server::control_port|
	{
		#info["starting query stream"];
		let notify_port = comm::port();
		let notify_chan = comm::chan(notify_port);
		
		let key = #fmt["query %?", ptr::addr_of(notify_port)];
		comm::send(state_chan, register_msg(name, key, query, notify_chan));
		
		let mut solution = ~[];
		loop
		{
			alt comm::select2(notify_port, control_port)
			{
				either::left(new_solution)
				{
					if new_solution != solution
					{
						solution = new_solution;	// TODO: need to escape the json?
						comm::send(push, #fmt["retry: 5000\ndata: %s\n\n", solution_to_json(solution).to_str()]);
					}
				}
				either::right(server::refresh_event)
				{
					comm::send(push, #fmt["retry: 5000\ndata: %s\n\n", solution_to_json(solution).to_str()]);
				}
				either::right(server::close_event)
				{
					#info["shutting down query stream"];
					comm::send(state_chan, deregister_msg(name, key));
					break;
				}
			}
		}
	}
}

fn solution_to_json(solution: solution) -> std::json::json
{
	std::json::list(@
		do vec::map(solution)
		|row|
		{
			solution_row_to_json(row)
		}
	)
}

fn solution_row_to_json(row: solution_row) -> std::json::json
{
	std::json::dict(
		std::map::hash_from_strs(
			do vec::map(row)
			|entry|
			{
				let (key, value) = entry;
				(key, object_to_json(value))
			}
		)
	)
}

// TODO: need to escape as html?
fn object_to_json(obj: object) -> std::json::json
{
	alt obj
	{
		iri_value(value) | blank_value(value)
		{
			value.to_json()
		}
		unbound_value(*) | invalid_value(*) | error_value(*)
		{
			// TODO: use custom css and render these in red
			obj.to_str().to_json()
		}
		string_value(value, ~"")
		{
			value.to_json()
		}
		_
		{
			obj.to_str().to_json()
		}
	}
}
