// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
export put_snmp;

fn add_string(&triples: [triple], subject: str, property: str, value: str)
{
	let object = primitive(string(value));
	vec::push(triples, {subject: iri(subject), property: property, object: object});
}

fn add_strings(&triples: [triple], subject: str, property: str, value: [str])
{
	let objects = vec::map(value) {|v| primitive(string(v))};
	vec::push(triples, {subject: iri(subject), property: property, object: seq(objects)});
}

fn add_str_default(&triples: [triple], subject: str, property: str, data: std::map::hashmap<str, std::json::json>, key: str, default: str)
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
					default
				}
			}
		}
		none
		{
			default
		}
	};
	add_string(triples, subject, property, object);
}

// We save the most important bits of data that we receive from json into gnos statements
// so that we can more easily model devices that don't use snmp.
fn add_device(&triples: [triple], subject: str, device: std::map::hashmap<str, std::json::json>)
{
	add_str_default(triples, subject, "gnos:name", device, "sysName", "<unknown>");	// TODO: admin property, if set, should override this
	add_str_default(triples, subject, "gnos:description", device, "sysDescr", "");			// TODO: admin property, if set, should override this
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn add_snmp(&triples: [triple], subject: str, object: std::map::hashmap<str, std::json::json>)
{
	for object.each()
	{|name, value|
		alt value
		{
			std::json::string(s)
			{
				add_string(triples, subject, name, s);
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
fn json_to_triples(remote_addr: str, data: std::json::json) -> [triple]
{
	let mut triples = [];
	
	// TODO: This should come from some data structure under the control of an admin.
	add_strings(triples, "gnos:admin", "gnos:managed-ips", ["10.101.100.2"]);
	
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
						add_device(triples, subject, device);
						
						let subject = #fmt["gnos:snmp-device-%s", managed_ip];
						add_snmp(triples, subject, device);
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
	for vec::each(triples)
	{|triple|
		#debug["   %s", triple.to_str()];
	};
	
	ret triples;
}

fn put_snmp(state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	// TODO:
	// have the home page show something about the json
	// test the error case
	
	// may want to verify that the json is a dict
	// convert the json into triples
	// send the triples to the state manager task
	// change home so that it shows the devices in the store
	// home should have (in admin/debug) a metric for the store size
	// commit and push rwebserve changes
	
	alt std::json::from_str(request.body)
	{
		result::ok(data)
		{
			#info["got new modeler data"];
			comm::send(state_chan, setter(json_to_triples(request.remote_addr, data)));
			{body: "" with response}
		}
		result::err(err)
		{
			let intro = #fmt["Malformed json on line %? col %?", err.line, err.col];
			#info["Error getting new modeler data:"];
			#info["%s: %s", intro, err.msg];
			
			response.context.insert("intro", mustache::str(intro));
			response.context.insert("error", mustache::str(err.msg));
			{status: "400 Bad Request", template: "bad-request" with response}	// not terribly useful to send html to scripts, but it might be handy to have the error visible in packets
		}
	}
}

