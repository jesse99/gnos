// Uses Server Sent Events to send solutions for a query after the model is updated.
import rrdf::*;
import std::json::to_json;
import std::json::to_str;

export get_query;

impl of to_json for object
{
	// TODO: need to escape as html
	fn to_json() -> std::json::json
	{
		alt self
		{
			iri_value(value) | blank_value(value)
			{
				let s = #fmt["<a href=\"%s\">%s</a>", value, value];
				s.to_json()
			}
			unbound_value(*) | invalid_value(*) | error_value(*)
			{
				// TODO: use custom css and render these in red
				self.to_str().to_json()
			}
			string_value(value, "")
			{
				value.to_json()
			}
			_
			{
				self.to_str().to_json()
			}
		}
	}
}

fn solution_row_to_json(row: solution_row) -> std::json::json
{
	std::json::dict(
		std::map::hash_from_strs(
			do vec::map(row)
			|entry|
			{
				let (key, value) = entry;
				(key, value.to_json())
			}
		)
	)
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

fn get_query(state_chan: comm::chan<msg>, _request: server::request, push: server::push_chan) -> server::control_chan
{
	do task::spawn_listener
	|control_port: server::control_port|
	{
		let query = "
PREFIX gnos: <http://www.gnos.org/2012/schema#>
SELECT DISTINCT
	?name
WHERE
{
	?subject ?predicate ?object .
	BIND(rrdf:pname(?subject) AS ?name)
} ORDER BY ?name";
		
		#info["starting query stream"];
		let notify_port = comm::port();
		let notify_chan = comm::chan(notify_port);
		
		let key = #fmt["query %?", ptr::addr_of(notify_port)];
		comm::send(state_chan, register_msg(key, query, notify_chan));
		
		let mut solution = ~[];
		loop
		{
			alt comm::select2(notify_port, control_port)
			{
				either::left(new_solution)
				{
					if new_solution != solution
					{
						solution = new_solution;	// TODO: need to escape the json
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
					comm::send(state_chan, deregister_msg(key));
					break;
				}
			}
		}
	}
}
