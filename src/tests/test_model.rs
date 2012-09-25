use std::json::ToJson;
use io::WriterUtil;
use rrdf::rrdf::*;
use model::*;
use Namespace = rrdf::solution::Namespace;

fn check_solutions(actual: &Solution, expected: &Solution) -> bool
{
	fn print_result(value: &Solution)
	{
		for vec::eachi(value.rows)
		|i, row|
		{
			let mut entries = ~[];
			for row.each |e| {vec::push(entries, fmt!("%s = %s", e.first(), e.second().to_str()))};
			io::stderr().write_line(fmt!("   %?: %s", i, str::connect(entries, ~", ")));
		};
	}
	
	fn print_failure(mesg: ~str, actual: &Solution, expected: &Solution)
	{
		io::stderr().write_line(mesg);
		io::stderr().write_line("Actual:");
		print_result(actual);
		io::stderr().write_line("Expected:");
		print_result(expected);
	}
	
	// OK if they are both empty.
	if vec::is_empty(actual.rows) && vec::is_empty(expected.rows)
	{
		return true;
	}
	
	// Both sides should have the same number of rows.
	if vec::len(actual.rows) != vec::len(expected.rows)
	{
		print_failure(#fmt["Actual result had %? rows but expected %? rows.", 
			vec::len(actual.rows), vec::len(expected.rows)], actual, expected);
		return false;
	}
	
	// Actual should have only the expected values.
	for vec::eachi(actual.rows)
	|i, row1|
	{
		let row2 = expected.rows[i];
		if vec::len(row1) != vec::len(row2)
		{
			print_failure(#fmt["Row %? had size %? but expected %?.",
				i, vec::len(row1), vec::len(row2)], actual, expected);
			return false;
		}
		
		for row1.each
		|entry1|
		{
			let name1 = entry1.first();
			let value1 = entry1.second();
			match row2.search(name1)
			{
				option::Some(value2) =>
				{
					if value1 != value2
					{
						print_failure(#fmt["Row %? actual %s was %s but expected %s.",
							i, name1, value1.to_str(), value2.to_str()], actual, expected);
						return false;
					}
				}
				option::None =>
				{
					print_failure(#fmt["Row %? had unexpected ?%s.",
						i, name1], actual, expected);
					return false;
				}
			}
		};
	};
	
	return true;
}

fn update(state_chan: comm::Chan<Msg>, data: ~[(~str, ~str)])
{
	fn get_str(entry: @~[std::json::Json], index: uint) -> ~str
	{
		match entry[index]
		{
			std::json::String(value) =>
			{
				*value
			}
			x =>
			{
				fail fmt!("Expected ~[str] but found %?", x)
			}
		}
	}
	
	fn do_update(store: &Store, data: ~str) -> bool
	{
		match std::json::from_str(data)
		{
			result::Ok(std::json::List(items)) =>
			{
				let subject = ~"http://blah";
				for items.each
				|item|
				{
					match *item
					{
						std::json::List(entry) =>
						{
							let key = get_str(entry, 0);
							let value = get_str(entry, 1);
							store.replace_triple(~[], {subject: subject, predicate: ~"sname:" + key, object: StringValue(value, ~"")});
						}
						y =>
						{
							fail fmt!("Expected ~[key, value] but found %?", y);
						}
					}
				}
				true
			}
			result::Ok(x) =>
			{
				fail fmt!("Expected list but found %?", x)
			}
			x =>
			{
				fail fmt!("Expected list but found %?", x)
			}
		}
	}
	
	let json = data.to_json();
	comm::send(state_chan, UpdateMsg(~"primary", do_update, json.to_str()));
}

#[test]
fn test_query()
{
	let state_chan = do utils::spawn_moded_listener(task::ManualThreads(2)) |port| {model::manage_state(port)};
	let sync_port = comm::Port();
	let sync_chan = comm::Chan(sync_port);
	
	let query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?ttl
WHERE
{
	?subject sname:ttl ?ttl
}";
	let query_port = comm::Port();
	let query_chan = comm::Chan(query_port);
	comm::send(state_chan, QueryMsg(~"primary", query, query_chan));
	
	// store starts out empty
	let solution = query_chan.recv();
	assert check_solutions(&solution, &Solution {namespaces: ~[], rows: ~[
	]});
	
	// after adding ttl can query for it
	update(state_chan, ~[(~"ttl", ~"50")]);
	comm::send(state_chan, QueryMsg(~"primary", query, query_chan));
	let solution = query_chan.recv();
	assert check_solutions(&solution, &Solution {namespaces: ~[], rows: ~[
		~[(~"ttl", StringValue(~"50", ~""))],
	]});
	
	// after changing ttl can query for it
	update(state_chan, ~[(~"ttl", ~"75")]);
	comm::send(state_chan, QueryMsg(~"primary", query, query_chan));
	let solution = query_chan.recv();
	assert check_solutions(&solution, &Solution {namespaces: ~[], rows: ~[
		~[(~"ttl", StringValue(~"75", ~""))],
	]});
	
	// only get a solution after a change if we request it
	update(state_chan, ~[(~"ttl", ~"80")]);
	comm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !query_chan.peek();
	
	comm::send(state_chan, ExitMsg);
}

#[test]
fn test_registration()
{
	let state_chan = do utils::spawn_moded_listener(task::ManualThreads(2)) |port| {model::manage_state(port)};
	let sync_port = comm::Port();
	let sync_chan = comm::Chan(sync_port);
	
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
	let ttl_port = comm::Port();
	let ttl_chan = comm::Chan(ttl_port);
	comm::send(state_chan, RegisterMsg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	let fwd_query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?fwd
WHERE
{
	?subject sname:fwd ?fwd
}";
	let fwd_port = comm::Port();
	let fwd_chan = comm::Chan(fwd_port);
	comm::send(state_chan, RegisterMsg(~"primary", ~"fwd-query", ~[fwd_query], fwd_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_solutions(&solutions[0], &Solution {namespaces: ~[], rows: ~[
	]});
	
	let solutions = fwd_chan.recv().get();
	assert check_solutions(&solutions[0], &Solution {namespaces: ~[], rows: ~[
	]});
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert solutions.len() == 1;
	assert check_solutions(&solutions[0], &Solution {namespaces: ~[], rows: ~[
		~[(~"ttl", StringValue(~"50", ~""))],
	]});
	
	comm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !fwd_chan.peek();
	
	// no solutions when replacing a triplet with the same triplet
	update(state_chan, ~[(~"ttl", ~"50")]);
	comm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// no solutions when adding a triplet the queries don't check
	update(state_chan, ~[(~"foo", ~"xx")]);
	task::yield();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// bail
	comm::send(state_chan, ExitMsg);
}

#[test]
fn test_deregistration()
{
	let state_chan = do utils::spawn_moded_listener(task::ManualThreads(2)) |port| {model::manage_state(port)};
	let sync_port = comm::Port();
	let sync_chan = comm::Chan(sync_port);
	
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
	let ttl_port = comm::Port();
	let ttl_chan = comm::Chan(ttl_port);
	comm::send(state_chan, RegisterMsg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_solutions(&solutions[0], &Solution {namespaces: ~[], rows: ~[
	]});
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert solutions.len() == 1;
	assert check_solutions(&solutions[0], &Solution {namespaces: ~[], rows: ~[
		~[(~"ttl", StringValue(~"50", ~""))],
	]});
	
	// but once we deregister we don't get solutions
	comm::send(state_chan, DeregisterMsg(~"primary", ~"ttl-query"));
	update(state_chan, ~[(~"ttl", ~"75")]);
	
	comm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	
	// bail
	comm::send(state_chan, ExitMsg);
	assert !ttl_chan.peek();
}

#[test]
fn test_alerts()
{
	fn check_alerts(store: &Store, expected: &Solution) -> bool 
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
		match eval_query(store, query)
		{
			result::Ok(actual) =>
			{
				check_solutions(&actual, expected)
			}
			result::Err(err) =>
			{
				fail err;
			}
		}
	}
	
	let namespaces = ~[
		Namespace {prefix: ~"devices", path: ~"http://network/"},
		Namespace {prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		Namespace {prefix: ~"snmp", path: ~"http://snmp/"},
		Namespace {prefix: ~"sname", path: ~"http://snmp-name/"},
	];
	let store = Store(namespaces, &std::map::HashMap());
	
	// open foo/bar => adds the alert
	open_alert(&store, Alert {device: ~"gnos:foo", id: ~"bar", level: ErrorLevel, mesg: ~"fie", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(false))],
	]});
	
	// open foo/bar => does nothing
	open_alert(&store, Alert {device: ~"gnos:foo", id: ~"bar", level: ErrorLevel, mesg: ~"no-op fie", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(false))],
	]});
	
	// open foo/cat => adds alert
	open_alert(&store, Alert {device: ~"gnos:foo", id: ~"cat", level: ErrorLevel, mesg: ~"meow", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"meow", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(false))],
	]});
	
	// close foo/bar => closes it
	close_alert(&store, ~"gnos:foo", ~"bar");
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"meow", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(true))],
	]});
	
	// close foo/dog => does nothing
	close_alert(&store, ~"gnos:foo", ~"dog");
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"meow", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(true))],
	]});
	
	// close foo/bar => does nothing
	close_alert(&store, ~"gnos:foo", ~"bar");
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"meow", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(true))],
	]});
	
	// open foo/bar => adds a new alert
	open_alert(&store, Alert {device: ~"gnos:foo", id: ~"bar", level: ErrorLevel, mesg: ~"fum", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], rows: ~[
		~[(~"mesg", StringValue(~"fum", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"meow", ~"")), (~"closed", BoolValue(false))],
		~[(~"mesg", StringValue(~"fie", ~"")), (~"closed", BoolValue(true))],
	]});
}
