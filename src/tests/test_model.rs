use core::path::{GenericPath};
use std::json::ToJson;
use io::WriterUtil;
use rrdf::*;
use model::*;
use Namespace = rrdf::solution::Namespace;

pub fn check_strs(actual: ~str, expected: ~str) -> bool
{
	if actual != expected
	{
		io::stderr().write_line(fmt!("Found '%s', but expected '%s'", actual, expected));
		return false;
	}
	return true;
}

fn check_solutions(actual: &Solution, expected: &Solution) -> bool
{
	fn print_result(value: &Solution)
	{
		for vec::eachi(value.rows) |i, row|
		{
			let mut entries = ~[];
			for row.eachi |j, e| {vec::push(&mut entries, fmt!("%s = %s", value.bindings[j], e.to_str()))};
			io::stderr().write_line(fmt!("   %?: %s", i, str::connect(entries, ~", ")));
		};
	}
	
	fn print_failure(mesg: &str, actual: &Solution, expected: &Solution)
	{
		io::stderr().write_line(mesg);
		io::stderr().write_line("Actual:");
		print_result(actual);
		io::stderr().write_line("Expected:");
		print_result(expected);
	}
	
	// OK if they are both empty.
	if actual.rows.is_empty() && expected.rows.is_empty()
	{
		return true;
	}
	
	// Both sides should have the same number of rows.
	if actual.rows.len() != expected.rows.len()
	{
		print_failure(fmt!("Actual result had %? rows but expected %? rows.", 
			actual.rows.len(), expected.rows.len()), actual, expected);
		return false;
	}
	
	// Actual should have only the expected values.
	for vec::eachi(actual.rows) |i, row1|
	{
		let row2 = copy expected.rows[i];
		if actual.num_selected != row2.len()
		{
			print_failure(fmt!("Row %? had size %? but expected %?.",
				i, row1.len(), row2.len()), actual, expected);
			return false;
		}
		
		for uint::range(0, actual.num_selected) |j|
		{
			let name1 = &actual.bindings[j];
			let value1 = row1[j];
			let value2 = row2[j];
			if value1 != value2
			{
				print_failure(fmt!("Row %? actual %s was %s but expected %s.",
					i, *name1, value1.to_str(), value2.to_str()), actual, expected);
				return false;
			}
		}
	}
	
	return true;
}

fn update(state_chan: oldcomm::Chan<Msg>, data: ~[(~str, ~str)])
{
	fn get_str(entry: &[std::json::Json], index: uint) -> ~str
	{
		match entry[index]
		{
			std::json::String(ref value) =>
			{
				value.to_owned()
			}
			ref x =>
			{
				fail fmt!("Expected ~[str] but found %?", x)
			}
		}
	}
	
	fn do_update(store: &Store, data: &str) -> bool
	{
		match std::json::from_str(data)
		{
			result::Ok(std::json::List(ref items)) =>
			{
				let subject = ~"http://blah";
				for items.each
				|item|
				{
					match *item
					{
						std::json::List(ref entry) =>
						{
							let key = get_str(*entry, 0);
							let value = get_str(*entry, 1);
							store.replace_triple(~[], {subject: copy subject, predicate: ~"gnos:" + key, object: @StringValue(value, ~"")});
						}
						ref y =>
						{
							fail fmt!("Expected ~[key, value] but found %?", y);
						}
					}
				}
				true
			}
			result::Ok(ref x) =>
			{
				fail fmt!("Expected list but found %?", x)
			}
			ref x =>
			{
				fail fmt!("Expected list but found %?", x)
			}
		}
	}
	
	let json = data.to_json();
	oldcomm::send(state_chan, UpdateMsg(~"primary", do_update, json.to_str()));
}

#[test]
fn test_query()
{
	let state_chan = do utils::spawn_moded_listener(task::ThreadPerCore) |port| {model::manage_state(port, "127.0.0.1", 8080)};
	let sync_port = oldcomm::Port();
	let sync_chan = oldcomm::Chan(&sync_port);
	
	let query = ~"
SELECT
	?ttl
WHERE
{
	?subject gnos:ttl ?ttl
}";
	let query_port = oldcomm::Port();
	let query_chan = oldcomm::Chan(&query_port);
	oldcomm::send(state_chan, QueryMsg(~"primary", copy query, query_chan));
	
	// store starts out empty
	let solution = query_chan.recv();
	assert check_strs(solution.to_str(), ~"[]");
	
	// after adding ttl can query for it
	update(state_chan, ~[(~"ttl", ~"50")]);
	oldcomm::send(state_chan, QueryMsg(~"primary", copy query, query_chan));
	let solution = query_chan.recv();
	assert check_strs(solution.to_str(), ~"[{\"ttl\":\"50\"}]");
	
	// after changing ttl can query for it
	update(state_chan, ~[(~"ttl", ~"75")]);
	oldcomm::send(state_chan, QueryMsg(~"primary", query, query_chan));
	let solution = query_chan.recv();
	assert check_strs(solution.to_str(), ~"[{\"ttl\":\"75\"}]");
	
	// only get a solution after a change if we request it
	update(state_chan, ~[(~"ttl", ~"80")]);
	oldcomm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !query_chan.peek();
	
	oldcomm::send(state_chan, ExitMsg);
}

#[test]
fn test_registration()
{
	let state_chan = do utils::spawn_moded_listener(task::ThreadPerCore) |port| {model::manage_state(port, "127.0.0.1", 8080)};
	let sync_port = oldcomm::Port();
	let sync_chan = oldcomm::Chan(&sync_port);
	
	// register queries
	let ttl_query = ~"
SELECT
	?ttl
WHERE
{
	?subject gnos:ttl ?ttl
}";
	let ttl_port = oldcomm::Port();
	let ttl_chan = oldcomm::Chan(&ttl_port);
	oldcomm::send(state_chan, RegisterMsg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	let fwd_query = ~"
SELECT
	?fwd
WHERE
{
	?subject gnos:fwd ?fwd
}";
	let fwd_port = oldcomm::Port();
	let fwd_chan = oldcomm::Chan(&fwd_port);
	oldcomm::send(state_chan, RegisterMsg(~"primary", ~"fwd-query", ~[fwd_query], fwd_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_strs(solutions.to_str(), ~"[]");
	
	let solutions = fwd_chan.recv().get();
	assert check_strs(solutions.to_str(), ~"[]");
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert check_strs(solutions.to_str(), ~"[{\"ttl\":\"50\"}]");
	
	oldcomm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !fwd_chan.peek();
	
	// no solutions when replacing a triplet with the same triplet
	update(state_chan, ~[(~"ttl", ~"50")]);
	oldcomm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// no solutions when adding a triplet the queries don't check
	update(state_chan, ~[(~"foo", ~"xx")]);
	task::yield();
	assert !ttl_chan.peek();
	assert !fwd_chan.peek();
	
	// bail
	oldcomm::send(state_chan, ExitMsg);
}

#[test]
fn test_deregistration()
{
	let state_chan = do utils::spawn_moded_listener(task::ThreadPerCore) |port| {model::manage_state(port, "127.0.0.1", 8080)};
	let sync_port = oldcomm::Port();
	let sync_chan = oldcomm::Chan(&sync_port);
	
	// register queries
	let ttl_query = ~"
SELECT
	?ttl
WHERE
{
	?subject gnos:ttl ?ttl
}";
	let ttl_port = oldcomm::Port();
	let ttl_chan = oldcomm::Chan(&ttl_port);
	oldcomm::send(state_chan, RegisterMsg(~"primary", ~"ttl-query", ~[ttl_query], ttl_chan));
	
	// get solutions on registration,
	let solutions = ttl_chan.recv().get();
	assert check_strs(solutions.to_str(), ~"[]");
	
	// and when the query results change
	update(state_chan, ~[(~"ttl", ~"50")]);
	let solutions = ttl_chan.recv().get();
	assert check_strs(solutions.to_str(), ~"[{\"ttl\":\"50\"}]");
	
	// but once we deregister we don't get solutions
	oldcomm::send(state_chan, DeregisterMsg(~"primary", ~"ttl-query"));
	update(state_chan, ~[(~"ttl", ~"75")]);
	
	oldcomm::send(state_chan, SyncMsg(sync_chan)); sync_chan.recv();
	assert !ttl_chan.peek();
	
	// bail
	oldcomm::send(state_chan, ExitMsg);
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
			result::Ok(ref actual) =>
			{
				check_solutions(actual, expected)
			}
			result::Err(ref err) =>
			{
				fail copy *err;
			}
		}
	}
	
	let namespaces = ~[
		Namespace {prefix: ~"devices", path: ~"http://network/"},
		Namespace {prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
		Namespace {prefix: ~"snmp", path: ~"http://snmp/"},
		Namespace {prefix: ~"gnos", path: ~"http://www.gnos.org/2012/schema#"},
	];
	let store = Store(namespaces, &std::map::HashMap());
	
	// open foo/bar => adds the alert
	open_alert(&store, &Alert {target: ~"gnos:foo", id: ~"bar", level: ~"0", mesg: ~"fie", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"fie", ~""), @BoolValue(false)],
	]});
	
	// open foo/bar => does nothing
	open_alert(&store, &Alert {target: ~"gnos:foo", id: ~"bar", level: ~"0", mesg: ~"no-op fie", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"fie", ~""), @BoolValue(false)],
	]});
	
	// open foo/cat => adds alert
	open_alert(&store, &Alert {target: ~"gnos:foo", id: ~"cat", level: ~"0", mesg: ~"meow", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"meow", ~""), @BoolValue(false)],
		~[@StringValue(~"fie", ~""), @BoolValue(false)],
	]});
	
	// close foo/bar => closes it
	close_alert(&store, ~"gnos:foo", ~"bar");
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"meow", ~""), @BoolValue(false)],
		~[@StringValue(~"fie", ~""), @BoolValue(true)],
	]});
	
	// close foo/dog => does nothing
	close_alert(&store, ~"gnos:foo", ~"dog");
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"meow", ~""), @BoolValue(false)],
		~[@StringValue(~"fie", ~""), @BoolValue(true)],
	]});
	
	// close foo/bar => does nothing
	close_alert(&store, ~"gnos:foo", ~"bar");
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"meow", ~""), @BoolValue(false)],
		~[@StringValue(~"fie", ~""), @BoolValue(true)],
	]});
	
	// open foo/bar => adds a new alert
	open_alert(&store, &Alert {target: ~"gnos:foo", id: ~"bar", level: ~"0", mesg: ~"fum", resolution: ~""});
	assert check_alerts(&store, &Solution {namespaces: ~[], bindings: ~[~"mesg", ~"closed", ~"subject", ~"end"], num_selected: 2, rows: ~[
		~[@StringValue(~"fum", ~""), @BoolValue(false)],
		~[@StringValue(~"meow", ~""), @BoolValue(false)],
		~[@StringValue(~"fie", ~""), @BoolValue(true)],
	]});
}
