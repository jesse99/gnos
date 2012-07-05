// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
import rrdf::object::*;
import rrdf::store::*;

export put_snmp;

fn lookup(table: std::map::hashmap<str, std::json::json>, key: str, default: str) -> str
{
	alt table.find(key)
	{
		option::some(std::json::string(s))
		{
			*s
		}
		option::some(value)
		{
			// This is something that should never happen so it's not so bad that we don't provide a lot of context
			// (if it does somehow happen admins can crank up the logging level to see where it is coming from).
			#error["%s was expected to be a string but is a %?", key, value];	// TODO: would be nice if the site could somehow show logs
			default
		}
		option::none
		{
			default
		}
	}
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn add_snmp(store: store, label: str, object: std::map::hashmap<str, std::json::json>) -> str
{
	let mut entries = [];
	vec::reserve(entries, object.size());
	
	for object.each()			// unfortunately hashmap doesn't support the base_iter protocol so there's no nice way to do this
	{|name, value|
		alt value
		{
			std::json::string(s)
			{
				vec::push(entries, ("snmp:" + name, string_value(*s, "")));
			}
			std::json::dict(_d)
			{
				// our caller should handle this with another call to add_smp
			}
			std::json::list(_l)
			{
				// this is the interfaces list (handled in add_device)
			}
			_
			{
				#error["%s was expected to contain string, dict, and list items but %s was %?", label, name, value];
			}
		}
	};
	
	let subject = get_blank_name(store, #fmt["%s-snmp", label]);
	store.add(subject, entries);
	ret subject;
}

 // "ifAdminStatus": "up(1)", 
  // "ifDescr": "eth3", 
  // "ifInDiscards": "74", 
  // "ifInOctets": "13762376", 
  // "ifInUcastPkts": "155115", 
  // "ifLastChange": "1503", 
  // "ifMtu": "1500", 
  // "ifOperStatus": "up(1)", 
  // "ifOutOctets": "12213444", 
  // "ifOutUcastPkts": "148232", 
  // "ifPhysAddress": "00:30:18:ab:0f:a1", 
  // "ifSpeed": "100000000", 
  // "ifType": "ethernetCsmacd(6)", 
  // "ipAdEntAddr": "10.101.3.2", 
  // "ipAdEntNetMask": "255.255.255.0"
fn add_interface(store: store, managed_ip: str, data: std::json::json) -> (str, object)
{
	alt data
	{
		std::json::dict(interface)
		{
			let name = lookup(interface, "ifDescr", "");
			let label = #fmt["%s-%s", managed_ip, name];
			
			let entries = [
				("gnos:ifname", string_value(name, "")),
				("gnos:ip", string_value(lookup(interface, "ipAdEntAddr", ""), "")),
				("gnos:netmask", string_value(lookup(interface, "ipAdEntNetMask", ""), "")),
				("gnos:mac", string_value(lookup(interface, "ifPhysAddress", ""), "")),
				("gnos:mtu", literal_to_object(lookup(interface, "ifMtu", ""), "http://www.w3.org/2001/XMLSchema#integer", "")),
				("gnos:enabled", bool_value(str::contains(lookup(interface, "ifOperStatus", ""), "(1)"))),	// TODO: verify that we want this and not ifAdminStatus
				("gnos:snmp", blank_value(add_snmp(store, label, interface))),
			];
			
			let subject = get_blank_name(store, label);
			store.add(subject, entries);
			("gnos:interface", blank_value(subject))
		}
		_
		{
			#error["Expected dict for %s interfaces but found %?", managed_ip, data];
			("gnos:missing-interface", string_value("", ""))
		}
	}
}

fn add_interfaces(store: store, managed_ip: str, device: std::map::hashmap<str, std::json::json>) -> [(str, object)]/~
{
	alt device["interfaces"]
	{
		std::json::list(interfaces)
		{
			vec::map(*interfaces)
			{|interface|
				add_interface(store, managed_ip, interface)
			}
		}
		_
		{
			#error["Expected list for %s interfaces but found %?", managed_ip, device.get("interfaces")];
			[]/~
		}
	}
}

// We save the most important bits of data that we receive from json into gnos statements
// so that we can more easily model devices that don't use snmp.
//
// "ipDefaultTTL": "64", 
// "ipForwDatagrams": "8", 
// "ipForwarding": "forwarding(1)", 
// "ipInDelivers": "338776", 
// "ipInReceives": "449623", 
// "ipNetToMediaType": "dynamic(3)", 
// "ipOutDiscards": "1", 
// "ipOutRequests": "325767", 
// "sysContact": "support@cococorp.com", 
// "sysDescr": "Linux GRS-A 2.6.39.4 #1 Wed Apr 4 02:43:16 PDT 2012 i686", 
// "sysLocation": "air", 
// "sysName": "GRS", 
// "sysUpTime": "5080354"
fn add_device(store: store, managed_ip: str, device: std::map::hashmap<str, std::json::json>)
{
	let entries = [
		("gnos:managed_ip", typed_value(managed_ip, "gnos:ip_address")),
		("gnos:name", string_value(lookup(device, "sysName", "unknown"), "")),	// TODO: admin property, if set, should override this
		("gnos:description", string_value(lookup(device, "sysDescr", ""), "")),		// TODO: admin property, if set, should override this
		("gnos:snmp", blank_value(add_snmp(store, managed_ip, device))),
	] + add_interfaces(store, managed_ip, device);
	
	let subject = get_blank_name(store, managed_ip);
	store.add(subject, entries);
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
			store.add_triple([]/~, {subject: "gnos:system", predicate: "gnos:last_update", object: dateTime_value(std::time::now())});
			
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
								add_device(store, managed_ip, device);
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
