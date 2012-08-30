import json::to_json;
import rrdf::{create_store, store, solution, get_blank_name, string_value, bool_value};
import rrdf::solution::{solution_row_trait};
import model::*;

fn check_solutions(actual: solution, expected: solution) -> bool
{
	fn print_result(value: solution)
	{
		for vec::eachi(value)
		|i, row|
		{
			let mut entries = ~[];
			for row.each |e| {vec::push(entries, #fmt["%s = %s", e.first(), e.second().to_str()])};
			io::stderr().write_line(#fmt["   %?: %s", i, str::connect(entries, ~", ")]);
		};
	}
	
	fn print_failure(mesg: ~str, actual: solution, expected: solution)
	{
		io::stderr().write_line(mesg);
		io::stderr().write_line("Actual:");
		print_result(actual);
		io::stderr().write_line("Expected:");
		print_result(expected);
	}
	
	// OK if they are both empty.
	if vec::is_empty(actual) && vec::is_empty(expected)
	{
		ret true;
	}
	
	// Both sides should have the same number of rows.
	if vec::len(actual) != vec::len(expected)
	{
		print_failure(#fmt["Actual result had %? rows but expected %? rows.", 
			vec::len(actual), vec::len(expected)], actual, expected);
		ret false;
	}
	
	// Actual should have only the expected values.
	for vec::eachi(actual)
	|i, row1|
	{
		let row2 = expected[i];
		if vec::len(row1) != vec::len(row2)
		{
			print_failure(#fmt["Row %? had size %? but expected %?.",
				i, vec::len(row1), vec::len(row2)], actual, expected);
			ret false;
		}
		
		for row1.each
		|entry1|
		{
			let name1 = entry1.first();
			let value1 = entry1.second();
			alt row2.search(name1)
			{
				option::some(value2)
				{
					if value1 != value2
					{
						print_failure(#fmt["Row %? actual %s was %s but expected %s.",
							i, name1, value1.to_str(), value2.to_str()], actual, expected);
						ret false;
					}
				}
				option::none
				{
					print_failure(#fmt["Row %? had unexpected ?%s.",
						i, name1], actual, expected);
					ret false;
				}
			}
		};
	};
	
	ret true;
}

fn update(state_chan: comm::chan<msg>, data: ~[(~str, ~str)])
{
	fn get_str(entry: @~[json::json], index: uint) -> ~str
	{
		alt entry[index]
		{
			json::string(value)
			{
				*value
			}
			x
			{
				fail #fmt["Expected ~[str] but found %?", x]
			}
		}
	}
	
	fn do_update(store: store, data: ~str) -> bool
	{
		alt json::from_str(data)
		{
			result::ok(json::list(items))
			{
				let subject = ~"http://blah";
				for items.each
				|item|
				{
					alt item
					{
						json::list(entry)
						{
							let key = get_str(entry, 0);
							let value = get_str(entry, 1);
							store.replace_triple(~[], {subject: subject, predicate: ~"sname:" + key, object: string_value(value, ~"")});
						}
						y
						{
							fail #fmt["Expected ~[key, value] but found %?", y];
						}
					}
				}
				true
			}
			result::ok(x)
			{
				fail #fmt["Expected list but found %?", x]
			}
			x
			{
				fail #fmt["Expected list but found %?", x]
			}
		}
	}
	
	let json = data.to_json();
	comm::send(state_chan, update_msg(~"primary", do_update, json.to_str()));
}

#[test]
fn test_query()
{
	let state_chan = do task::spawn_listener |port| {model::manage_state(port)};
	let sync_port = comm::port();
	let sync_chan = comm::chan(sync_port);
	
	let query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?ttl
WHERE
{
	?subject sname:ttl ?ttl
}";
	let query_port = comm::port();
	let query_chan = comm::chan(query_port);
	comm::send(state_chan, query_msg(~"primary", query, query_chan));
	
	// store starts out empty
	let solution = query_chan.recv();
	assert check_solutions(solution, ~[
	]);
	
	// after adding ttl can query for it
	update(state_chan, ~[(~"ttl", ~"50")]);
	comm::send(state_chan, query_msg(~"primary", query, query_chan));
	let solution = query_chan.recv();
	assert check_solutions(solution, ~[
		~[(~"ttl", string_value(~"50", ~""))],
	]);
	
	// after changing ttl can query for it
	update(state_chan, ~[(~"ttl", ~"75")]);
	comm::send(state_chan, query_msg(~"primary", query, query_chan));
	let solution = query_chan.recv();
	assert check_solutions(solution, ~[
		~[(~"ttl", string_value(~"75", ~""))],
	]);
	
	// only get a solution after a change if we request it
	update(state_chan, ~[(~"ttl", ~"80")]);
	comm::send(state_chan, sync_msg(sync_chan)); sync_chan.recv();
	assert !query_chan.peek();
	
	comm::send(state_chan, exit_msg);
}

#[test]
fn test_registration()
{
	let state_chan = do task::spawn_listener |port| {model::manage_state(port)};
	let sync_port = comm::port();
	let sync_chan = comm::chan(sync_port);
	
	// register queries
	let ttl_query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?ttl
WHERE
{
	?subject sname:ttl ?ttl
}";
	let ttl_port = comm::port();
	let ttl_chan = comm::chan(ttl_port);
	comm::send(state_chan, register_msg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	let fwd_query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?fwd
WHERE
{
	?subject sname:fwd ?fwd
}";
	let fwd_port = comm::port();
	let fwd_chan = comm::chan(fwd_port);
	comm::send(state_chan, register_msg(~"primary", ~"fwd-query", ~[fwd_query], fwd_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_solutions(solutions[0], ~[
	]);
	
	let solutions = fwd_chan.recv().get();
	assert check_solutions(solutions[0], ~[
	]);
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert solutions.len() == 1;
	assert check_solutions(solutions[0], ~[
		~[(~"ttl", string_value(~"50", ~""))],
	]);
	
	comm::send(state_chan, sync_msg(sync_chan)); sync_chan.recv();
	assert !fwd_chan.peek();
	
	// no solutions when replacing a triplet with the same triplet
	update(state_chan, ~[(~"ttl", ~"50")]);
	comm::send(state_chan, sync_msg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// no solutions when adding a triplet the queries don't check
	update(state_chan, ~[(~"foo", ~"xx")]);
	task::yield();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// bail
	comm::send(state_chan, exit_msg);
}

#[test]
fn test_deregistration()
{
	let state_chan = do task::spawn_listener |port| {model::manage_state(port)};
	let sync_port = comm::port();
	let sync_chan = comm::chan(sync_port);
	
	// register queries
	let ttl_query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?ttl
WHERE
{
	?subject sname:ttl ?ttl
}";
	let ttl_port = comm::port();
	let ttl_chan = comm::chan(ttl_port);
	comm::send(state_chan, register_msg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_solutions(solutions[0], ~[
	]);
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert solutions.len() == 1;
	assert check_solutions(solutions[0], ~[
		~[(~"ttl", string_value(~"50", ~""))],
	]);
	
	// but once we deregister we don't get solutions
	comm::send(state_chan, deregister_msg(~"primary", ~"ttl-query"));
	update(state_chan, ~[(~"ttl", ~"75")]);
	
	comm::send(state_chan, sync_msg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	
	// bail
	comm::send(state_chan, exit_msg);
	assert !ttl_chan.peek();
}

#[test]
fn test_alerts()
{
	fn check_alerts(store: store, expected: solution) -> bool 
	{
		let query = ~"PREFIX devices: <http://network/>
			PREFIX gnos: <http://www.gnos.org/2012/schema#>
			SELECT
				?mesg ?closed
			WHERE
			{
				?subject gnos:mesg ?mesg .
				OPTIONAL
				{
					?subject gnos:end ?end
				}
				BIND(BOUND(?end) AS ?closed) 
			}";
		alt eval_query(store, query)
		{
			result::ok(actual)
			{
				check_solutions(actual, expected)
			}
			result::err(err)
			{
				fail err;
			}
		}
	}
	
	let namespaces = ~[
		{prefix: ~"devices", path: ~"http://network/"},
		{prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		{prefix: ~"snmp", path: ~"http://snmp/"},
		{prefix: ~"sname", path: ~"http://snmp-name/"},
	];
	let store = create_store(namespaces, @std::map::str_hash());
	
	// open foo/bar => adds the alert
	open_alert(store, {device: ~"gnos:foo", id: ~"bar", level: error_level, mesg: ~"fie", resolution: ~""});
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(false))],
	]);
	
	// open foo/bar => does nothing
	open_alert(store, {device: ~"gnos:foo", id: ~"bar", level: error_level, mesg: ~"no-op fie", resolution: ~""});
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(false))],
	]);
	
	// open foo/cat => adds alert
	open_alert(store, {device: ~"gnos:foo", id: ~"cat", level: error_level, mesg: ~"meow", resolution: ~""});
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(false))],
		~[(~"mesg", string_value(~"meow", ~"")), (~"closed", bool_value(false))],
	]);
	
	// close foo/bar => closes it
	close_alert(store, ~"gnos:foo", ~"bar");
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(true))],
		~[(~"mesg", string_value(~"meow", ~"")), (~"closed", bool_value(false))],
	]);
	
	// close foo/dog => does nothing
	close_alert(store, ~"gnos:foo", ~"dog");
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(true))],
		~[(~"mesg", string_value(~"meow", ~"")), (~"closed", bool_value(false))],
	]);
	
	// close foo/bar => does nothing
	close_alert(store, ~"gnos:foo", ~"bar");
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(true))],
		~[(~"mesg", string_value(~"meow", ~"")), (~"closed", bool_value(false))],
	]);
	
	// open foo/bar => adds a new alert
	open_alert(store, {device: ~"gnos:foo", id: ~"bar", level: error_level, mesg: ~"fum", resolution: ~""});
	assert check_alerts(store, ~[
		~[(~"mesg", string_value(~"fie", ~"")), (~"closed", bool_value(true))],
		~[(~"mesg", string_value(~"meow", ~"")), (~"closed", bool_value(false))],
		~[(~"mesg", string_value(~"fum", ~"")), (~"closed", bool_value(false))],
	]);
}
