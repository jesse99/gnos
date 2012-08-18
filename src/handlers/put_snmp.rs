// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
import model::{msg, update_msg, query_msg};
import options::{options, device};
import rrdf::{store, string_value, get_blank_name, object, literal_to_object, bool_value, float_value, blank_value, typed_value,
	iri_value, int_value, dateTime_value, solution, solution_row};
import rrdf::solution::{solution_row_trait};
import rrdf::store::{base_iter, store_trait};

export put_snmp;

fn put_snmp(options: options, state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	let addr = request.remote_addr;
	#info["got new modeler data from %s", addr];
	
	// Arguably cleaner to do this inside of json_to_store (or add_device) but we'll deadlock if we try
	// to do a query inside of an update_mesg callback.
	let old = query_old_info(state_chan);
	
	let ooo = copy(options);
	comm::send(state_chan, update_msg(~"primary", |s, d| {json_to_store(ooo, addr, s, d, old)}, request.body));
	{body: ~"" with response}
}

fn query_old_info(state_chan: comm::chan<msg>) -> solution
{
	let po = comm::port();
	let ch = comm::chan(po);
	
	let query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
SELECT
	?subject ?old_timestamp ?old_ipInReceives ?old_ipForwDatagrams ?old_ipInDelivers
WHERE
{
	?subject gnos:old_timestamp ?old_timestamp .
	?subject gnos:old_ipInReceives ?old_ipInReceives .
	?subject gnos:old_ipForwDatagrams ?old_ipForwDatagrams .
	?subject gnos:old_ipInDelivers ?old_ipInDelivers .
}";
	
	comm::send(state_chan, query_msg(~"primary", query, ch));
	comm::recv(po)
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
fn json_to_store(options: options, remote_addr: ~str, store: store, body: ~str, old: solution) -> bool
{
	alt std::json::from_str(body)
	{
		result::ok(data)
		{
			store.clear();
			store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:last_update", object: dateTime_value(std::time::now())});
			store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:poll_interval", object: int_value(options.poll_rate as i64)});
			
			alt data
			{
				std::json::dict(d)
				{
					for d.each()
					|managed_ip, the_device|
					{
						alt the_device
						{
							std::json::dict(device)
							{
								add_device(store, options.devices, managed_ip, device, old);
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
			
			#info["Received data from %s:", remote_addr];
			//for store.each {|triple| #debug["   %s", triple.to_str()];};
			true
		}
		result::err(err)
		{
			let intro = #fmt["Malformed json on line %? col %? from %s", err.line, err.col, remote_addr];
			#error["Error getting new modeler data:"];
			#error["%s: %s", intro, *err.msg];
			false
		}
	}
}

// We save the most important bits of data that we receive from json into gnos statements
// so that we can more easily model devices that don't use snmp.
//
// "ipDefaultTTL": "64", 
// "ipForwDatagrams": "8", 
// "ipForwarding": "forwarding(1)", 
// "ipInDelivers": "338776", 				TODO: a lot of this has been deprecated, see http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipInDelivers&translate=Translate&submitValue=SUBMIT&submitClicked=true
// "ipInReceives": "449623", 
// "ipNetToMediaType": "dynamic(3)", 
// "ipOutDiscards": "1", 
// "ipOutRequests": "325767", 
// "sysContact": "support@cococorp.com", 
// "sysDescr": "Linux GRS-A 2.6.39.4 #1 Wed Apr 4 02:43:16 PDT 2012 i686", 
// "sysLocation": "air", 
// "sysName": "GRS", 
// "sysUpTime": "5080354"
fn add_device(store: store, devices: ~[device], managed_ip: ~str, device: std::map::hashmap<~str, std::json::json>, old: solution)
{
	alt devices.find(|d| {d.managed_ip == managed_ip})
	{
		option::some(jdevice)
		{
			let old_subject = option::some(iri_value(~"http://network/" + managed_ip));
			let old_row = old.find(|r| {r.search(~"subject") == old_subject});
			
			let entries = ~[
				(~"gnos:center_x", float_value(jdevice.center_x as f64)),
				(~"gnos:center_y", float_value(jdevice.center_y as f64)),
				(~"gnos:style", string_value(jdevice.style, ~"")),
				
				(~"gnos:primary_label", string_value(jdevice.name, ~"")),
				(~"gnos:secondary_label", string_value(managed_ip, ~"")),
				(~"gnos:tertiary_label", string_value(get_device_label(device, old_row).trim(), ~"")),
				
				// These are undocumented because they not intended to be used by clients.
				(~"gnos:old_timestamp", float_value(utils::imprecise_time_s() as f64)),
				(~"gnos:old_ipInReceives", int_value(get_snmp_i64(device, ~"ipInReceives", 0))),
				(~"gnos:old_ipForwDatagrams", int_value(get_snmp_i64(device, ~"ipForwDatagrams", 0))),
				(~"gnos:old_ipInDelivers", int_value(get_snmp_i64(device, ~"ipInDelivers", 0))),
			];
			
			let subject = #fmt["devices:%s", managed_ip];
			store.add(subject, entries);
		}
		option::none
		{
			#error["Couldn't find %s in the network json file", managed_ip];
		}
	};
}

fn get_device_label(device: std::map::hashmap<~str, std::json::json>, old: option::option<solution_row>) -> ~str
{
	let old_timestamp = if old.is_some() {old.get().get(~"old_timestamp").as_f64()} else {0.0 as f64};
	let delta_s = utils::imprecise_time_s() as f64 - old_timestamp;
	
	get_device_label_component(device, ~"ipInReceives", ~"recv", old, delta_s) +
	get_device_label_component(device, ~"ipForwDatagrams", ~"for", old, delta_s) +
	get_device_label_component(device, ~"ipInDelivers", ~"del", old, delta_s)
}

fn get_device_label_component(device: std::map::hashmap<~str, std::json::json>, key: ~str, label: ~str, old: option::option<solution_row>, delta_s: f64) -> ~str
{
	alt lookup(device, key, ~"")
	{
		~""
		{
			~""
		}
		value
		{
			let new_value = i64::from_str(value).get() as f64;
			let new_str_value = utils::i64_to_unit_str(new_value as i64);
			
			let old_value = if old.is_some() {old.get().get(~"old_" + key).as_f64()} else {0.0 as f64};
			if old_value > 0.0f64
			{
				let pps = (new_value - old_value)/delta_s;
				let pps_str_value = utils::i64_to_unit_str(pps as i64);
				#fmt["%s: %sp %spps\n", label, new_str_value, pps_str_value]
			}
			else
			{
				#fmt["%s: %sp\n", label, new_str_value]
			}
		}
	}
}

//fn add_interfaces(store: store, managed_ip: ~str, device: std::map::hashmap<~str, std::json::json>) -> ~[(~str, object)]
//{
//	alt device[~"interfaces"]
//	{
//		std::json::list(interfaces)
//		{
//			do vec::map(*interfaces)
//			|interface|
//			{
//				add_interface(store, managed_ip, interface)
//			}
//		}
//		_
//		{
//			#error["Expected list for %s interfaces but found %?", managed_ip, device.get(~"interfaces")];
//			~[]
//		}
//	}
//}

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
//fn add_interface(store: store, managed_ip: ~str, data: std::json::json) -> (~str, object)
//{
//	alt data
//	{
//		std::json::dict(interface)
//		{
//			let name = lookup(interface, ~"ifDescr", ~"");
//			let label = #fmt["%s-%s", managed_ip, name];
//			
//			let entries = ~[
//				(~"gnos:ifname", string_value(name, ~"")),
//				(~"gnos:ip", string_value(lookup(interface, ~"ipAdEntAddr", ~""), ~"")),
//				(~"gnos:netmask", string_value(lookup(interface, ~"ipAdEntNetMask", ~""), ~"")),
//				(~"gnos:mac", string_value(lookup(interface, ~"ifPhysAddress", ~""), ~"")),
//				(~"gnos:mtu", literal_to_object(lookup(interface, ~"ifMtu", ~""), ~"http://www.w3.org/2001/XMLSchema#integer", ~"")),
//				(~"gnos:enabled", bool_value(str::contains(lookup(interface, ~"ifOperStatus", ~""), ~"(1)"))),	// TODO: verify that we want this and not ifAdminStatus
//				(~"gnos:snmp", blank_value(add_snmp(store, label, interface))),
//			];
//			
//			let subject = get_blank_name(store, label);
//			store.add(subject, entries);
//			(~"gnos:interface", blank_value(subject))
//		}
//		_
//		{
//			#error["Expected dict for %s interfaces but found %?", managed_ip, data];
//			(~"gnos:missing-interface", string_value(~"", ~""))
//		}
//	}
//}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
//fn add_snmp(store: store, label: ~str, object: std::map::hashmap<~str, std::json::json>) -> ~str
//{
//	let mut entries = ~[];
//	vec::reserve(entries, object.size());
//	
//	for object.each()			// unfortunately hashmap doesn't support the base_iter protocol so there's no nice way to do this
//	|name, value|
//	{
//		alt value
//		{
//			std::json::string(s)
//			{
//				vec::push(entries, (~"snmp:" + name, string_value(*s, ~"")));
//			}
//			std::json::dict(_d)
//			{
//				// our caller should handle this with another call to add_smp
//			}
//			std::json::list(_l)
//			{
//				// this is the interfaces list (handled in add_device)
//			}
//			_
//			{
//				#error["%s was expected to contain string, dict, and list items but %s was %?", label, name, value];
//			}
//		}
//	};
//	
//	let subject = #fmt["%s-snmp", label];
//	store.add(subject, entries);
//	ret subject;
//}

fn get_snmp_i64(table: std::map::hashmap<~str, std::json::json>, key: ~str, default: i64) -> i64
{
	alt lookup(table, key, ~"")
	{
		~""
		{
			default
		}
		text
		{
			alt i64::from_str(text)
			{
				option::some(value)
				{
					value
				}
				option::none
				{
					#error["%s was %s, but expected an int", key, text];
					default
				}
			}
		}
	}
}

// Lookup an SNMP value.
fn lookup(table: std::map::hashmap<~str, std::json::json>, key: ~str, default: ~str) -> ~str
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

