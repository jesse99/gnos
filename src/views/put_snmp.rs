// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
import rrdf::object::*;
import rrdf::store::*;

export put_snmp;

fn add_string(store: store, subject: str, property: str, value: str)
{
	store.add_triple([]/~, make_triple_str(store, subject, property, value));
}

fn add_strings(store: store, subject: str, value: [str])
{
	let values = vec::map(value) {|v| literal_to_object(v, "http://www.w3.org/2001/XMLSchema#string", "")};
	store.add_seq(subject, values);
}

fn add_str_default(store: store, subject: str, property: str, data: std::map::hashmap<str, std::json::json>, 
	key: str, default: str)
{
	let object = alt data.find(key)
	{
		some(value)
		{
			alt value
			{
				std::json::string(s)
				{
					s
				}
				_
				{
					// This is something that should never happen so it's not so bad that we don't provide a lot of context
					// (if it does somehow happen admins can crank up the logging level to see where it is coming from).
					#error["%s was expected to be a string but is a %?", key, value];	// TODO: would be nice if the site could somehow show logs
					@default
				}
			}
		}
		none
		{
			@default
		}
	};
	add_string(store, subject, property, *object);
}

// We save the most important bits of data that we receive from json into gnos statements
// so that we can more easily model devices that don't use snmp.
fn add_device(store: store, subject: str, device: std::map::hashmap<str, std::json::json>)
{
	add_str_default(store, subject, "gnos:name", device, "sysName", "<unknown>");	// TODO: admin property, if set, should override this
	add_str_default(store, subject, "gnos:description", device, "sysDescr", "");			// TODO: admin property, if set, should override this
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn add_snmp(store: store, subject: str, object: std::map::hashmap<str, std::json::json>)
{
	for object.each()
	{|name, value|
		alt value
		{
			std::json::string(s)
			{
				add_string(store, subject, name, *s);
			}
			std::json::dict(_d)
			{
				// our caller should handle this with another call to add_smp
			}
			std::json::list(_l)
			{
				// this is the interfaces list
				// TODO: can probably nuke this once we start processing the interfaces
			}
			_
			{
				#error["%s was expected to contain string, dict, and list items but had a %?", subject, value];
			}
		}
	};
}

// Format is:
// {
//    "10.101.100.2": 			managed ip address
//    {
//        "ipBlah": "23",			primitive values are all strings
//        ...
//        "interfaces": 
//        [
//            {,
//               "ifBlah": "foo",
//                ...
//            },
//            ...
//        ]
//    },
//    ...
// }
fn json_to_store(remote_addr: str, store: store, body: str)
{
	alt std::json::from_str(body)
	{
		result::ok(data)
		{
			store.clear();
			
			// TODO: This should come from some data structure under the control of an admin.
			add_strings(store, "gnos:managed-ips", ["10.101.100.2"]);
			
			alt data
			{
				std::json::dict(d)
				{
					for d.each()
					{|managed_ip, the_device|
						alt the_device
						{
							std::json::dict(device)
							{
								let subject = #fmt["gnos:device-%s", managed_ip];
								add_device(store, subject, device);
								
								let subject = #fmt["gnos:snmp-device-%s", managed_ip];
								add_snmp(store, subject, device);
							}
							_
							{
								#error["%s device from %s was expected to be a dict but is a %?", managed_ip, remote_addr, the_device];	// TODO: probably want to add errors to store
							}
						}
					};
				}
				_
				{
					#error["Data from %s was expected to be a dict but is a %?", remote_addr, data];	// TODO: probably want to add errors to store
				}
			}
			
			#debug["Data received from %s:", remote_addr];
			for store.each
			{|triple|
				#debug["   %s", triple.to_str()];
			};
		}
		result::err(err)
		{
			let intro = #fmt["Malformed json on line %? col %? from %s", err.line, err.col, remote_addr];
			#error["Error getting new modeler data:"];
			#error["%s: %s", intro, *err.msg];
		}
	}
}

fn put_snmp(state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	#info["got new modeler data"];
	let addr = request.remote_addr;
	comm::send(state_chan, setter({|s, d| json_to_store(addr, s, d)}, request.body));
	{body: "" with response}
}

