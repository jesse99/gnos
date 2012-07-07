// TODO: Should be able to simply import rrdf. See https://github.com/mozilla/rust/issues/1935
import rrdf::*;

// Data can be anything, but is typically json.
type store_setter = fn~ (store: store, data: str) -> ();

enum msg
{
	getter(str, comm::chan<solution>),		// SPARQL query + channel to send results back along
	setter(store_setter, str)						// function to use to update the store + data to use
}

// In general the same queries will be used over and over again so it will be
// much more efficient to cache the selectors.
fn get_selector(queries: hashmap<str, selector>, query: str) -> option::option<selector>
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
					#error["Failed to compile: %s", err];
					option::none
				}
			}
		}
	}
}

// The manage_state function runs within a dedicated task and allows
// other tasks to get a snapshot of the model or update the model.
fn manage_state(port: comm::port<msg>)
{
	let queries = std::map::str_hash();
	let store = create_store(
		[
			{prefix: "gnos", path: "http://www.gnos.org/2012/schema#"},
			{prefix: "snmp", path: "http://www.gnos.org/2012/snmp/"},
		]/~, []/~);
	
	loop
	{
		alt comm::recv(port)
		{
			getter(query, channel)
			{
				let s = get_selector(queries, query);
				if option::is_some(s)
				{
					alt option::get(s)(store)
					{
						result::ok(rows)
						{
							comm::send(channel, copy(rows));
						}
						result::err(err)
						{
							#error["'%s' failed with %s", query, err];
							comm::send(channel, []/~);
						}
					}
				}
			}
			setter(f, data)
			{
				f(store, data);
				#info["Updated store"];
				//for store.each {|triple| #info["%s", triple.to_str()];};
			}
		}
	}
}

fn get_state(channel: comm::chan<msg>, query: str) -> solution
{
	let port = comm::port::<solution>();
	let chan = comm::chan::<solution>(port);
	comm::send(channel, getter(query, chan));
	let result = comm::recv(port);
	ret result;
}
