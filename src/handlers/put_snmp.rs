// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
import core::to_str::{to_str};
import core::dvec::*;
import model::{msg, update_msg, updates_msg, query_msg, eval_query};
import options::{options, device};
import rrdf::{store, string_value, get_blank_name, object, literal_to_object, bool_value, float_value, blank_value, typed_value,
	iri_value, int_value, dateTime_value, solution, solution_row};
import rrdf::solution::{solution_row_trait};
import rrdf::store::{base_iter, store_trait, triple, to_str};

export put_snmp;

fn put_snmp(options: options, state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	let addr = request.remote_addr;
	#info["got new modeler data from %s", addr];
	
	// Arguably cleaner to do this inside of json_to_store (or add_device) but we'll deadlock if we try
	// to do a query inside of an updates_mesg callback.
	let old = query_old_info(state_chan);
	
	let ooo = copy(options);
	comm::send(state_chan, updates_msg(~[~"primary", ~"snmp", ~"alerts"], |ss, d| {updates_snmp(ooo, addr, ss, d, old)}, request.body));
	
	{body: ~"" with response}
}

fn updates_snmp(options: options, remote_addr: ~str, stores: ~[store], body: ~str, old: solution) -> bool
{
	alt std::json::from_str(body)
	{
		result::ok(data)
		{
			alt data
			{
				std::json::dict(d)
				{
					json_to_primary(options, remote_addr, stores[0], stores[2], d, old);
					json_to_snmp(remote_addr, stores[1], d);
				}
				_
				{
					#error["Data from %s was expected to be a dict but is a %?", remote_addr, data];	// TODO: probably want to add errors to store
				}
			}
		}
		result::err(err)
		{
			let intro = #fmt["Malformed json on line %? col %? from %s", err.line, err.col, remote_addr];
			#error["Error getting new modeler data:"];
			#error["%s: %s", intro, *err.msg];
		}
	}
	
	true
}

fn query_old_info(state_chan: comm::chan<msg>) -> solution
{
	let po = comm::port();
	let ch = comm::chan(po);
	
	let query = ~"
PREFIX gnos: <http://www.gnos.org/2012/schema#>
PREFIX sname: <http://snmp-name/>
SELECT
	?subject ?name ?value
WHERE
{
	?subject gnos:internal-info ?old .
	?old ?predicate ?value .
	BIND(rrdf:pname(?predicate) AS ?name) 
}";
	
	comm::send(state_chan, query_msg(~"primary", query, ch));
	let solution = comm::recv(po);
	//for solution.eachi |i, r| {#error["%?: %?", i, r]}
	solution
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
fn json_to_primary(options: options, remote_addr: ~str, store: store, alerts_store: store, data: std::map::hashmap<~str, json::json>, old: solution)
{
	store.clear();
	store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:last_update", object: dateTime_value(std::time::now())});
	store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:poll_interval", object: int_value(options.poll_rate as i64)});
	
	for data.each()
	|managed_ip, the_device|
	{
		alt the_device
		{
			std::json::dict(device)
			{
				let old_subject = get_blank_name(store, ~"old");
				add_device(store, alerts_store, options.devices, managed_ip, device, old, old_subject);
				add_device_notes(store, alerts_store, managed_ip, device);
			}
			_
			{
				#error["%s device from %s was expected to be a dict but is a %?", managed_ip, remote_addr, the_device];	// TODO: probably want to add errors to store
			}
		}
	};
	
	#info["Received data from %s:", remote_addr];
	//for store.each |triple| {#info["   %s", triple.to_str()];};
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
// "sysContact": "support@xyz.com", 
// "sysDescr": "Linux Router-A 2.6.39.4 #1 Wed Apr 4 02:43:16 PDT 2012 i686", 
// "sysLocation": "closet", 
// "sysName": "Router", 
// "sysUpTime": "5080354"
fn add_device(store: store, alerts_store: store, devices: ~[device], managed_ip: ~str, device: std::map::hashmap<~str, std::json::json>, old: solution, old_subject: ~str)
{
	alt devices.find(|d| {d.managed_ip == managed_ip})
	{
		option::some(options_device)
		{
			let entries = ~[
				(~"gnos:center_x", float_value(options_device.center_x as f64)),
				(~"gnos:center_y", float_value(options_device.center_y as f64)),
				(~"gnos:style", string_value(options_device.style, ~"")),
				
				(~"gnos:primary_label", string_value(options_device.name, ~"")),
				(~"gnos:secondary_label", string_value(managed_ip, ~"")),
				(~"gnos:tertiary_label", string_value(get_device_label(device, managed_ip, old).trim(), ~"")),
			];
			let subject = #fmt["devices:%s", managed_ip];
			store.add(subject, entries);
			
			// These are undocumented because they not intended to be used by clients.
			store.add_triple(~[], {subject: subject, predicate: ~"gnos:internal-info", object: blank_value(old_subject)});
			
			let entries = ~[
				(~"gnos:timestamp", float_value(utils::imprecise_time_s() as f64)),
				(~"sname:ipInReceives", int_value(get_snmp_i64(device, ~"ipInReceives", 0))),
				(~"sname:ipForwDatagrams", int_value(get_snmp_i64(device, ~"ipForwDatagrams", 0))),
				(~"sname:ipInDelivers", int_value(get_snmp_i64(device, ~"ipInDelivers", 0))),
			];
			store.add(old_subject, entries);
			
			toggle_device_uptime_alert(alerts_store, managed_ip, device);
			
			let interfaces = device.find(~"interfaces");
			if interfaces.is_some()
			{
				let has_interfaces = add_interfaces(store, alerts_store, managed_ip, interfaces.get(), old, old_subject);
				toggle_device_down_alert(alerts_store, managed_ip, has_interfaces);
			}
			else
			{
				toggle_device_down_alert(alerts_store, managed_ip, false);
			}
		}
		option::none
		{
			#error["Couldn't find %s in the network json file", managed_ip];
		}
	};
}

fn toggle_device_uptime_alert(alerts_store: store, managed_ip: ~str, device: std::map::hashmap<~str, std::json::json>)
{
	let time = (get_snmp_i64(device, ~"sysUpTime", -1) as float)/100.0;
	let device = #fmt["devices:%s", managed_ip];
	let id = ~"uptime";
	
	if time >= 0.0 && time < 60.0		// only reboot if we actually got an up time
	{
		// TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
		let mesg = ~"Device rebooted.";		// we can't add the time here because alerts aren't changed when re-opened (and the mesg doesn't change when they are closed)
		model::open_alert(alerts_store, {device: device, id: id, level: model::warning_level, mesg: mesg, resolution: ~""});
	}
	else
	{
		model::close_alert(alerts_store, device, id);
	}
}

fn toggle_device_down_alert(alerts_store: store, managed_ip: ~str, up: bool)
{
	let device = #fmt["devices:%s", managed_ip];
	let id = ~"down";
	
	if up
	{
		model::close_alert(alerts_store, device, id);
	}
	else
	{
		let mesg = ~"Device is down.";
		let resolution = ~"Check the power cable, power it on if it is off, check the IP address, verify routing.";
		model::open_alert(alerts_store, {device: device, id: id, level: model::error_level, mesg: mesg, resolution: resolution});
	}
}

fn add_device_notes(store: store, alerts_store: store, managed_ip: ~str, _device: std::map::hashmap<~str, std::json::json>)
{
	// summary
	let html = #fmt["
<p class='summary'>
	The name and ip address are from the network json file. All the other info is from <a href='./subject/snmp/snmp:%s'>SNMP</a>.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipInReceives&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Received </a> is the number of packets received on interfaces.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipForwDatagrams&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Forwarded </a> is the number of packets received but not destined for the device.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipInDelivers&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Delivered </a> is the number of packets sent to a local IP protocol.
</p>", managed_ip];
	
	let subject = get_blank_name(store, ~"summary");
	store.add(subject, ~[
		(~"gnos:title",       string_value(~"notes", ~"")),
		(~"gnos:target",    iri_value(#fmt["devices:%s", managed_ip])),
		(~"gnos:detail",    string_value(html, ~"")),
		(~"gnos:weight",  float_value(0.1f64)),
		(~"gnos:open",     string_value(~"no", ~"")),
		(~"gnos:key",       string_value(~"device notes", ~"")),
	]);
	
	// alerts
	for get_alert_html(alerts_store, managed_ip).each
	|level, alerts|
	{
		add_alerts(store, managed_ip, alerts.get(), level);
	}
}

fn add_alerts(store: store, managed_ip: ~str, alerts: ~[(float, ~str)], level: ~str)
{
	if alerts.is_not_empty()
	{
		let alerts = std::sort::merge_sort(|x, y| {x <= y}, alerts);
		
		let mut html = ~"";
		html += ~"<ul class = 'sequence'>\n";
			let items = do alerts.map |r| {#fmt["<li>%s</li>\n", r.second()]};
			html += str::connect(items, ~"");
		html += ~"</ul>\n";
		
		let weight = 
			alt level
			{
				~"error" {0.9f64}
				~"warning" {0.8f64}
				~"info" {0.3f64}
				_ {0.01f64}
			};
		
		let subject = get_blank_name(store, #fmt["%s %s-alert", managed_ip, level]);
		store.add(subject, ~[
			(~"gnos:title",       string_value(level + ~" alerts", ~"")),
			(~"gnos:target",    iri_value(#fmt["devices:%s", managed_ip])),
			(~"gnos:detail",    string_value(html, ~"")),
			(~"gnos:weight",  float_value(weight)),
			(~"gnos:open",     string_value(if level == ~"error" {~"always"} else {~"no"}, ~"")),
			(~"gnos:key",       string_value(level + ~"alert", ~"")),
		]);
	}
}

fn get_alert_html(alerts_store: store, managed_ip: ~str) -> std::map::hashmap<~str, @dvec<(float, ~str)>>
{
	let table = std::map::str_hash();		// level => [(elapsed, html)]
	table.insert(~"error", @dvec());
	table.insert(~"warning", @dvec());
	table.insert(~"info", @dvec());
	table.insert(~"debug", @dvec());
	table.insert(~"closed", @dvec());
	
	// Show all open alerts and all alerts closed within the last seven days.
	let now = std::time::get_time();
	let then = {sec: now.sec - 60*60*24*7 with now};
	
	let device = #fmt["devices:%s", managed_ip];
	let expr = #fmt["
	PREFIX devices: <http://network/>
	PREFIX gnos: <http://www.gnos.org/2012/schema#>
	PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
	SELECT
		?begin ?mesg ?level ?end ?resolution
	WHERE
	{
		?subject gnos:device %s .
		?subject gnos:begin ?begin .
		?subject gnos:mesg ?mesg .
		?subject gnos:level ?level .
		?subject gnos:resolution ?resolution .
		OPTIONAL
		{
			?subject gnos:end ?end
		}
		FILTER (!bound(?end) || ?end >= \"%s\"^^xsd:dateTime)
	}", device, std::time::at_utc(then).rfc3339()];
	
	alt eval_query(alerts_store, expr)
	{
		result::ok(solution)
		{
			for solution.each
			|row|
			{
				let level = row.get(~"level").as_str();
				let level = if row.contains(~"end") {~"closed"} else {level};
				
				let begin = row.get(~"begin").as_tm();
				let {elapsed, delta} = utils::tm_to_delta_str(begin);
				
				let mesg = row.get(~"mesg").as_str();
				let (elapsed, mesg) =
					if !row.contains(~"end")
					{
						(elapsed, if elapsed > 60.0{#fmt["%s (%s)", mesg, delta]} else {mesg})
					}
					else
					{
						let end = row.get(~"end").as_tm();
						let {elapsed, delta} = utils::tm_to_delta_str(end);
						(elapsed, if elapsed > 60.0{#fmt["%s (closed %s)", mesg, delta]} else {mesg})
					};
				
				let klass = level + ~"-alert";
				let resolution = row.get(~"resolution").as_str();
				let html =
					if resolution.is_not_empty()
					{
						#fmt["<p class='%s tooltip' data-tooltip=' %s'>%s</p>", klass, resolution, mesg]
					}
					else
					{
						#fmt["<span class='%s'>%s</span>", klass, mesg]
					};
				table[level].push((elapsed, html));
			}
		}
		result::err(err)
		{
			#error["error querying for %s alerts: %s", managed_ip, err];
		}
	}
	
	ret table;
}

fn get_device_label(device: std::map::hashmap<~str, std::json::json>, managed_ip: ~str, old: solution) -> ~str
{
	let old_url = option::some(iri_value(~"http://network/" + managed_ip));
	
	let old_timestamp = get_old_f64(old_url, ~"gnos:timestamp", old);
	let delta_s = utils::imprecise_time_s() as f64 - old_timestamp;
	
	~"recv: " + get_per_second_value(device, ~"ipInReceives", old_url, ~"sname:ipInReceives", old, delta_s, ~"p") +
	~"fwd: " + get_per_second_value(device, ~"ipForwDatagrams", old_url, ~"sname:ipForwDatagrams", old, delta_s, ~"p") +
	~"del: " + get_per_second_value(device, ~"ipInDelivers", old_url, ~"sname:ipInDelivers", old, delta_s, ~"p")
}

fn add_interfaces(store: store, alerts_store: store, managed_ip: ~str, data: std::json::json, old: solution, old_subject: ~str) -> bool
{
	alt data
	{
		std::json::list(interfaces)
		{
			let mut rows = ~[];		// [(ifname, html)]
			for interfaces.each
			|interface|
			{
				alt interface
				{
					std::json::dict(d)
					{
						vec::push(rows, add_interface(store, alerts_store, managed_ip, d, old, old_subject));
					}
					_
					{
						#error["interface from device %s was expected to be a dict but is %?", managed_ip, interface];
					}
				}
			}
			let rows = std::sort::merge_sort(|lhs, rhs| {lhs.first() <= rhs.first()}, rows);
			let hrows = do rows.map |r| {r.second()};
			
			let mut html = ~"";
			html += ~"<table border='1' class = 'details'>\n";
				html += ~"<tr>\n";
					html += ~"<th>Name</th>\n";
					html += ~"<th>IP Address</th>\n";
					html += ~"<th>In Bytes</th>\n";
					html += ~"<th>Out Bytes</th>\n";
					html += ~"<th>Speed</th>\n";
					html += ~"<th>MAC Address</th>\n";
					html += ~"<th>MTU</th>\n";
					html += ~"<th>SNMP</th>\n";
				html += ~"</tr>\n";
				html += str::connect(hrows, ~"\n");
			html += ~"</table>\n";
			
			let subject = get_blank_name(store, ~"interfaces");
			store.add(subject, ~[
				(~"gnos:title",       string_value(~"interfaces", ~"")),
				(~"gnos:target",    iri_value(#fmt["devices:%s", managed_ip])),
				(~"gnos:detail",    string_value(html, ~"")),
				(~"gnos:weight",  float_value(0.8f64)),
				(~"gnos:open",     string_value(~"no", ~"")),
				(~"gnos:key",       string_value(~"interfaces", ~"")),
			]);
			
			interfaces.is_not_empty()
		}
		_
		{
			#error["interfaces from device %s was expected to be a list but is %?", managed_ip, data];
			false
		}
	}
}

// "ifAdminStatus": "up(1)", 
// "ifDescr": "eth3", 
// "ifInDiscards": "74", 
// "ifInOctets": "13762376", 
// "ifInUcastPkts": "155115", 
// "ifLastChange": "1503", 			didn't always see this one
// "ifMtu": "1500", 
// "ifOperStatus": "up(1)", 
// "ifOutOctets": "12213444", 
// "ifOutUcastPkts": "148232", 
// "ifPhysAddress": "00:30:18:ab:0f:a1", 
// "ifSpeed": "100000000", 
// "ifType": "ethernetCsmacd(6)", 
// "ipAdEntAddr": "10.101.3.2", 
// "ipAdEntNetMask": "255.255.255.0"
fn add_interface(store: store, alerts_store: store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::json>, old: solution, old_subject: ~str) -> (~str, ~str)
{
	let mut html = ~"";
	let name = lookup(interface, ~"ifDescr", ~"eth?");
	
	let oper_status = lookup(interface, ~"ifOperStatus", ~"missing");
	if oper_status.contains(~"(1)")
	{
		let prefix = #fmt["sname:%s-", name];
		
		let old_url = option::some(iri_value(~"http://network/" + managed_ip));
		let old_timestamp = get_old_f64(old_url, ~"gnos:timestamp", old);
		let delta_s = utils::imprecise_time_s() as f64 - old_timestamp;
		
		// TODO: We're not showing ifInUcastPkts and ifOutUcastPkts because bandwidth seems
		// more important, the table starts to get cluttered when we do, and multicast is at least as
		// important (to me anyway). I think what we should do is have a link somewhere that
		// displays a big chart allowing the client to pick which interfaces to display and which
		// traffic types (of course we'd also have to rely on either some other MIB or something
		// like Netflow).
		html += ~"<tr>\n";
			html += #fmt["<td>%s</td>", name];
			html += #fmt["<td>%s%s</td>", get_str_cell(interface, ~"ipAdEntAddr"), get_subnet(interface)];
			html += #fmt["<td>%s</td>", get_per_second_value(interface, ~"ifInOctets", old_url, prefix + ~"ifInOctets", old, delta_s, ~"b")];
			html += #fmt["<td>%s</td>", get_per_second_value(interface, ~"ifOutOctets", old_url, prefix + ~"ifOutOctets", old, delta_s, ~"b")];
			html += #fmt["<td>%s</td>", get_int_cell(interface, ~"ifSpeed", ~"bps")];
			html += #fmt["<td>%s</td>", get_str_cell(interface, ~"ifPhysAddress").to_upper()];
			html += #fmt["<td>%s</td>", get_int_cell(interface, ~"ifMtu", ~"B")];
			html += #fmt["<td><a href='./subject/snmp/snmp:%s-%s'>data</a></td>", managed_ip, name];
		html += ~"\n</tr>\n";
		
		// These are undocumented because they are not intended to be used by clients.
		let entries = ~[
			(prefix + ~"ifInOctets", int_value(get_snmp_i64(interface, ~"ifInOctets", 0))),
			(prefix + ~"ifOutOctets", int_value(get_snmp_i64(interface, ~"ifOutOctets", 0))),
		];
		store.add(old_subject, entries);
	}
	
	toggle_admin_vs_oper_interface_alert(alerts_store, managed_ip, interface, name, oper_status);
	toggle_weird_interface_state_alert(alerts_store, managed_ip, name, oper_status);
	
	ret (name, html);
}

fn toggle_admin_vs_oper_interface_alert(alerts_store: store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::json>, name: ~str, oper_status: ~str)
{
	let admin_status = lookup(interface, ~"ifAdminStatus", ~"");
	
	let device = #fmt["devices:%s", managed_ip];
	let id = name + ~"-status";
	if admin_status.is_not_empty() && oper_status != admin_status
	{
		let mesg = #fmt["Admin set %s to %s, but operational state is %s.", name, admin_status, oper_status];
		model::open_alert(alerts_store, {device: device, id: id, level: model::error_level, mesg: mesg, resolution: ~""});
	}
	else
	{
		model::close_alert(alerts_store, device, id);
	}
}

// Add a warning if an interface state is not up and not down, i.e. one of:
// 3 : testing
// 4 : unknown
// 5 : dormant			the interface is waiting for external actions (such as a serial line waiting for an incoming connection)
// 6 : notPresent
// 7 : lowerLayerDown
fn toggle_weird_interface_state_alert(alerts_store: store, managed_ip: ~str, name: ~str, oper_status: ~str)
{
	let device = #fmt["devices:%s", managed_ip];
	let id = name + ~"-weird";
	if oper_status.contains(~"(1)") || oper_status.contains(~"(2)")
	{
		model::close_alert(alerts_store, device, id);
	}
	else
	{
		let mesg = #fmt["%s operational state is %s.", name, oper_status];
		model::open_alert(alerts_store, {device: device, id: id, level: model::warning_level, mesg: mesg, resolution: ~""});
	}
}

fn get_subnet(interface: std::map::hashmap<~str, std::json::json>) -> ~str
{
	alt lookup(interface, ~"ipAdEntNetMask", ~"")
	{
		~""
		{
			~"/?"
		}
		s
		{
			let parts = s.split_char('.');
			let bytes = do parts.map |p| {uint::from_str(p).get()};		// TODO: probably shouldn't fail for malformed json
			let mask = do bytes.foldl(0) |sum, current| {256*sum + current};
			let leading = count_leading_ones(mask);
			let trailing = count_trailing_zeros(mask);
			if leading + trailing == 32
			{
				#fmt["/%?", leading]
			}
			else
			{
				// Unusual netmask where 0s and 1s are mixed.
				#fmt["/%s", s]
			}
		}
	}
}

#[test]
fn test_get_subnet()
{
	let interface = std::map::str_hash();
	assert get_subnet(interface) == ~"/?";
	
	interface.insert(~"ipAdEntNetMask", json::string(@~"255.255.255.255"));
	assert get_subnet(interface) == ~"/32";
	
	interface.insert(~"ipAdEntNetMask", json::string(@~"255.0.0.0"));
	assert get_subnet(interface) == ~"/8";
	
	interface.insert(~"ipAdEntNetMask", json::string(@~"0.0.0.0"));
	assert get_subnet(interface) == ~"/0";
	
	interface.insert(~"ipAdEntNetMask", json::string(@~"255.0.1.0"));
	assert get_subnet(interface) == ~"/255.0.1.0";
}

fn count_leading_ones(mask: uint) -> int
{
	let mut count = 0;
	
	let mut bit = 1u << 31;
	while bit > 0
	{
		if mask & bit == bit
		{
			count += 1;
			bit >>= 1;
		}
		else
		{
			break;
		}
	}
	
	ret count;
}

fn count_trailing_zeros(mask: uint) -> int
{
	let mut count = 0;
	
	let mut bit = 1u;
	while bit < 1u << 32
	{
		if mask & bit == 0
		{
			count += 1;
			bit <<= 1;
		}
		else
		{
			break;
		}
	}
	
	ret count;
}

fn get_per_second_value(data: std::map::hashmap<~str, std::json::json>, key: ~str, old_url: option::option<object>, name: ~str, old: solution, delta_s: f64, unit: ~str) -> ~str
{
	alt lookup(data, key, ~"")
	{
		~""
		{
			~"\n"
		}
		value
		{
			let new_value = i64::from_str(value).get() as f64;
			let new_str_value = utils::f64_to_unit_str(new_value);
			
			let old_value = get_old_f64(old_url, name, old);
			if old_value > 0.0f64 && delta_s > 1.0f64
			{
				// Showing the absolute packet numbers is nearly useless so we'll only
				// show packets per second if it is available.
				let pps = (new_value - old_value)/delta_s;
				let pps_str_value = utils::f64_to_unit_str(pps);
				#fmt["%s%sps\n", pps_str_value, unit]
			}
			else
			{
				#fmt["%s%s\n", new_str_value, unit]
			}
		}
	}
}

fn get_old_f64(old_url: option::option<object>, predicate: ~str, old: solution) -> f64
{
	let old_row = old.find(|r| {r.search(~"subject") == old_url && r.search(~"name") == option::some(string_value(predicate, ~""))});
	if old_row.is_some()
	{
		old_row.get().get(~"value").as_f64()
	}
	else
	{
		0.0f64
	}
}

fn get_scaled_int_value(data: std::map::hashmap<~str, std::json::json>, label: ~str, key: ~str, units: ~str, scaling: i64) -> ~str
{
	let value = get_snmp_i64(data, key, 0)*scaling;
	if value > 0
	{
		#fmt["<strong>%s:</strong> %s%s<br>\n", label, utils::i64_to_unit_str(value), units]
	}
	else
	{
		~""
	}
}

fn get_int_cell(data: std::map::hashmap<~str, std::json::json>, key: ~str, units: ~str) -> ~str
{
	let value = get_snmp_i64(data, key, 0);
	if value > 0
	{
		#fmt["%s%s", utils::i64_to_unit_str(value), units]
	}
	else
	{
		~""
	}
}

fn get_str_cell(data: std::map::hashmap<~str, std::json::json>, key: ~str) -> ~str
{
	lookup(data, key, ~"")
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn json_to_snmp(remote_addr: ~str, store: store, data: std::map::hashmap<~str, json::json>)
{
	store.clear();
	
	for data.each
	|key, value|
	{
		alt value
		{
			std::json::dict(d)
			{
				device_to_snmp(remote_addr, store, key, d);
			}
			_
			{
				#error["%s was expected to have a device map but %s was %?", remote_addr, key, value];
			}
		}
	}
}

fn device_to_snmp(remote_addr: ~str, store: store, managed_ip: ~str, data: std::map::hashmap<~str, json::json>)
{
	let mut entries = ~[];
	vec::reserve(entries, data.size());
	
	for data.each		// unfortunately hashmap doesn't support the base_iter protocol so there's no nice way to do this
	|name, value|
	{
		alt value
		{
			std::json::string(s)
			{
				vec::push(entries, (~"sname:" + name, string_value(*s, ~"")));
			}
			std::json::list(interfaces)
			{
				interfaces_to_snmp(remote_addr, store, managed_ip, interfaces);
			}
			_
			{
				#error["%s device was expected to contain string or list but %s was %?", remote_addr, name, value];
			}
		}
	};
	
	let subject = #fmt["snmp:%s", managed_ip];
	store.add(subject, entries);
}

fn interfaces_to_snmp(remote_addr: ~str, store: store, managed_ip: ~str, interfaces: @~[json::json])
{
	for interfaces.each
	|data|
	{
		alt data
		{
			std::json::dict(interface)
			{
				interface_to_snmp(remote_addr, store, managed_ip, interface);
			}
			_
			{
				#error["%s interfaces was expected to contain string or list but found %?", remote_addr, data];
			}
		}
	}
}

fn interface_to_snmp(remote_addr: ~str, store: store, managed_ip: ~str, interface: std::map::hashmap<~str, json::json>)
{
	let mut ifname = ~"";
	let mut entries = ~[];
	vec::reserve(entries, interface.size());
	
	for interface.each
	|name, value|
	{
		alt value
		{
			std::json::string(s)
			{
				if name == ~"ifDescr"
				{
					ifname = *s;
				}
				vec::push(entries, (~"sname:" + name, string_value(*s, ~"")));
			}
			_
			{
				#error["%s interfaces was expected to contain a string or dict but %s was %?", remote_addr, name, value];
			}
		}
	};
	
	if ifname.is_not_empty()
	{
		let subject = #fmt["snmp:%s", managed_ip + "-" + ifname];
		store.add(subject, entries);
	}
	else
	{
		#error["%s interface was missing an ifDescr:", remote_addr];
		for interface.each() |k, v| {#error["   %s => %?", k, v];};
	}
}

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

