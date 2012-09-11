// This is the code that handles PUTs from the snmp-modeler script. It parses the
// incoming json, converts it into triplets, and updates the model.
//use core::to_str::{to_str};
use core::dvec::*;
use model::{Msg, UpdateMsg, UpdatesMsg, QueryMsg, eval_query};
use options::{Options, Device};
use rrdf::rrdf::*;
use server = rwebserve::rwebserve;

export put_snmp;

fn put_snmp(options: Options, state_chan: comm::Chan<Msg>, request: &server::Request, response: &server::Response) -> server::Response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	let addr = request.remote_addr;
	info!("got new modeler data from %s", addr);
	
	// Arguably cleaner to do this inside of json_to_store (or add_device) but we'll deadlock if we try
	// to do a query inside of an updates_mesg callback.
	let old = query_old_info(state_chan);
	
	let ooo = copy(options);
	comm::send(state_chan, UpdatesMsg(~[~"primary", ~"snmp", ~"alerts"], |ss, d| {updates_snmp(ooo, addr, ss, d, &old)}, request.body));
	
	server::Response {body: ~"", ..*response}
}

fn updates_snmp(options: Options, remote_addr: ~str, stores: &[Store], body: ~str, old: &Solution) -> bool
{
	match std::json::from_str(body)
	{
		result::Ok(data) =>
		{
			match data
			{
				std::json::Dict(d) =>
				{
					json_to_primary(options, remote_addr, &stores[0], &stores[2], d, old);
					json_to_snmp(remote_addr, &stores[1], d);
				}
				_ =>
				{
					error!("Data from %s was expected to be a dict but is a %?", remote_addr, data);	// TODO: probably want to add errors to store
				}
			}
		}
		result::Err(err) =>
		{
			let intro = fmt!("Malformed json on line %? col %? from %s", err.line, err.col, remote_addr);
			error!("Error getting new modeler data:");
			error!("%s: %s", intro, *err.msg);
		}
	}
	
	true
}

fn query_old_info(state_chan: comm::Chan<Msg>) -> Solution
{
	let po = comm::Port();
	let ch = comm::Chan(po);
	
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
	
	comm::send(state_chan, QueryMsg(~"primary", query, ch));
	let solution = comm::recv(po);
	//for solution.eachi |i, r| {error!("%?: %?", i, r)}
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
fn json_to_primary(options: Options, remote_addr: ~str, store: &Store, alerts_store: &Store, data: std::map::hashmap<~str, std::json::Json>, old: &Solution)
{
	store.clear();
	store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:last_update", object: DateTimeValue(std::time::now())});
	store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:poll_interval", object: IntValue(options.poll_rate as i64)});
	
	for data.each()
	|managed_ip, the_device|
	{
		match the_device
		{
			std::json::Dict(device) =>
			{
				let old_subject = get_blank_name(store, ~"old");
				add_device(store, alerts_store, options.devices, managed_ip, device, old, old_subject);
				add_device_notes(store, alerts_store, managed_ip, device);
			}
			_ =>
			{
				error!("%s device from %s was expected to be a dict but is a %?", managed_ip, remote_addr, the_device);	// TODO: probably want to add errors to store
			}
		}
	};
	
	info!("Received data from %s:", remote_addr);
	//for store.each |triple| {info!("   %s", triple.to_str());};
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
fn add_device(store: &Store, alerts_store: &Store, devices: ~[Device], managed_ip: ~str, device: std::map::hashmap<~str, std::json::Json>, old: &Solution, old_subject: ~str)
{
	match devices.find(|d| {d.managed_ip == managed_ip})
	{
		option::Some(options_device) =>
		{
			let time = (get_snmp_i64(device, ~"sysUpTime", -1) as float)/100.0;
			let entries = ~[
				(~"gnos:center_x", FloatValue(options_device.center_x as f64)),
				(~"gnos:center_y", FloatValue(options_device.center_y as f64)),
				(~"gnos:style", StringValue(options_device.style, ~"")),
				
				(~"gnos:primary_label", StringValue(options_device.name, ~"")),
				(~"gnos:secondary_label", StringValue(managed_ip, ~"")),
				(~"gnos:tertiary_label", StringValue(get_device_label(device, managed_ip, old, time).trim(), ~"")),
			];
			let subject = fmt!("devices:%s", managed_ip);
			store.add(subject, entries);
			
			// These are undocumented because they not intended to be used by clients.
			store.add_triple(~[], {subject: subject, predicate: ~"gnos:internal-info", object: BlankValue(old_subject)});
			
			let entries = ~[
				(~"gnos:timestamp", FloatValue(time as f64)),
				(~"sname:ipInReceives", IntValue(get_snmp_i64(device, ~"ipInReceives", 0))),
				(~"sname:ipForwDatagrams", IntValue(get_snmp_i64(device, ~"ipForwDatagrams", 0))),
				(~"sname:ipInDelivers", IntValue(get_snmp_i64(device, ~"ipInDelivers", 0))),
			];
			store.add(old_subject, entries);
			
			toggle_device_uptime_alert(alerts_store, managed_ip, time);
			
			let interfaces = device.find(~"interfaces");
			if interfaces.is_some()
			{
				let has_interfaces = add_interfaces(store, alerts_store, managed_ip, interfaces.get(), old, old_subject, time);
				toggle_device_down_alert(alerts_store, managed_ip, has_interfaces);
			}
			else
			{
				toggle_device_down_alert(alerts_store, managed_ip, false);
			}
		}
		option::None =>
		{
			error!("Couldn't find %s in the network json file", managed_ip);
		}
	};
}

fn toggle_device_uptime_alert(alerts_store: &Store, managed_ip: ~str, time: float)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = ~"uptime";
	
	if time >= 0.0 && time < 60.0		// only reboot if we actually got an up time
	{
		// TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
		let mesg = ~"Device rebooted.";		// we can't add the time here because alerts aren't changed when re-opened (and the mesg doesn't change when they are closed)
		model::open_alert(alerts_store, {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
	}
	else
	{
		model::close_alert(alerts_store, device, id);
	}
}

fn toggle_device_down_alert(alerts_store: &Store, managed_ip: ~str, up: bool)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = ~"down";
	
	if up
	{
		model::close_alert(alerts_store, device, id);
	}
	else
	{
		let mesg = ~"Device is down.";
		let resolution = ~"Check the power cable, power it on if it is off, check the IP address, verify routing.";
		model::open_alert(alerts_store, {device: device, id: id, level: model::ErrorLevel, mesg: mesg, resolution: resolution});
	}
}

fn add_device_notes(store: &Store, alerts_store: &Store, managed_ip: ~str, _device: std::map::hashmap<~str, std::json::Json>)
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
		(~"gnos:title",       StringValue(~"notes", ~"")),
		(~"gnos:target",    IriValue(fmt!("devices:%s", managed_ip))),
		(~"gnos:detail",    StringValue(html, ~"")),
		(~"gnos:weight",  FloatValue(0.1f64)),
		(~"gnos:open",     StringValue(~"no", ~"")),
		(~"gnos:key",       StringValue(~"device notes", ~"")),
	]);
	
	// alerts
	for get_alert_html(alerts_store, managed_ip).each
	|level, alerts|
	{
		add_alerts(store, managed_ip, alerts.get(), level);
	}
}

fn add_alerts(store: &Store, managed_ip: ~str, alerts: ~[(float, ~str)], level: ~str)
{
	if alerts.is_not_empty()
	{
		let alerts = std::sort::merge_sort(|x, y| {*x <= *y}, alerts);
		
		let mut html = ~"";
		html += ~"<ul class = 'sequence'>\n";
			let items = do alerts.map |r| {fmt!("<li>%s</li>\n", r.second())};
			html += str::connect(items, ~"");
		html += ~"</ul>\n";
		
		let weight = 
			match level
			{
				~"error" => {0.9f64}
				~"warning" => {0.8f64}
				~"info" => {0.3f64}
				_ => {0.01f64}
			};
		
		let subject = get_blank_name(store, fmt!("%s %s-alert", managed_ip, level));
		store.add(subject, ~[
			(~"gnos:title",       StringValue(level + ~" alerts", ~"")),
			(~"gnos:target",    IriValue(fmt!("devices:%s", managed_ip))),
			(~"gnos:detail",    StringValue(html, ~"")),
			(~"gnos:weight",  FloatValue(weight)),
			(~"gnos:open",     StringValue(if level == ~"error" {~"always"} else {~"no"}, ~"")),
			(~"gnos:key",       StringValue(level + ~"alert", ~"")),
		]);
	}
}

fn get_alert_html(alerts_store: &Store, managed_ip: ~str) -> std::map::hashmap<~str, @DVec<(float, ~str)>>
{
	let table = std::map::str_hash();		// level => [(elapsed, html)]
	table.insert(~"error", @DVec());
	table.insert(~"warning", @DVec());
	table.insert(~"info", @DVec());
	table.insert(~"debug", @DVec());
	table.insert(~"closed", @DVec());
	
	// Show all open alerts and all alerts closed within the last seven days.
	let now = std::time::get_time();
	let then = {sec: now.sec - 60*60*24*7 , .. now};
	
	let device = fmt!("devices:%s", managed_ip);
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
	
	match eval_query(alerts_store, expr)
	{
		result::Ok(solution) =>
		{
			for solution.rows.each
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
						(elapsed, if elapsed > 60.0{fmt!("%s (%s)", mesg, delta)} else {mesg})
					}
					else
					{
						let end = row.get(~"end").as_tm();
						let {elapsed, delta} = utils::tm_to_delta_str(end);
						(elapsed, if elapsed > 60.0{fmt!("%s (closed %s)", mesg, delta)} else {mesg})
					};
				
				let klass = level + ~"-alert";
				let resolution = row.get(~"resolution").as_str();
				let html =
					if resolution.is_not_empty()
					{
						fmt!("<p class='%s tooltip' data-tooltip=' %s'>%s</p>", klass, resolution, mesg)
					}
					else
					{
						fmt!("<span class='%s'>%s</span>", klass, mesg)
					};
				table[level].push((elapsed, html));
			}
		}
		result::Err(err) =>
		{
			error!("error querying for %s alerts: %s", managed_ip, err);
		}
	}
	
	return table;
}

fn get_device_label(device: std::map::hashmap<~str, std::json::Json>, managed_ip: ~str, old: &Solution, uptime: float) -> ~str
{
	let old_url = option::Some(IriValue(~"http://network/" + managed_ip));
	
	let old_timestamp = get_old_f64(old_url, ~"gnos:timestamp", old);
	let delta_s = uptime as f64 - old_timestamp;
	
	~"recv: " + get_per_second_str(device, ~"ipInReceives", old_url, ~"sname:ipInReceives", old, delta_s, ~"p") +
	~"fwd: " + get_per_second_str(device, ~"ipForwDatagrams", old_url, ~"sname:ipForwDatagrams", old, delta_s, ~"p") +
	~"del: " + get_per_second_str(device, ~"ipInDelivers", old_url, ~"sname:ipInDelivers", old, delta_s, ~"p")
}

fn add_interfaces(store: &Store, alerts_store: &Store, managed_ip: ~str, data: std::json::Json, old: &Solution, old_subject: ~str, uptime: float) -> bool
{
	match data
	{
		std::json::List(interfaces) =>
		{
			let mut rows = ~[];		// [(ifname, html)]
			for interfaces.each
			|interface|
			{
				match interface
				{
					std::json::Dict(d) =>
					{
						vec::push(rows, add_interface(store, alerts_store, managed_ip, d, old, old_subject, uptime));
					}
					_ =>
					{
						error!("interface from device %s was expected to be a dict but is %?", managed_ip, interface);
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
				(~"gnos:title",       StringValue(~"interfaces", ~"")),
				(~"gnos:target",    IriValue(fmt!("devices:%s", managed_ip))),
				(~"gnos:detail",    StringValue(html, ~"")),
				(~"gnos:weight",  FloatValue(0.8f64)),
				(~"gnos:open",     StringValue(~"no", ~"")),
				(~"gnos:key",       StringValue(~"interfaces", ~"")),
			]);
			
			interfaces.is_not_empty()
		}
		_ =>
		{
			error!("interfaces from device %s was expected to be a list but is %?", managed_ip, data);
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
fn add_interface(store: &Store, alerts_store: &Store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::Json>, old: &Solution, old_subject: ~str, uptime: float) -> (~str, ~str)
{
	let mut html = ~"";
	let name = lookup(interface, ~"ifDescr", ~"eth?");
	
	let oper_status = lookup(interface, ~"ifOperStatus", ~"missing");
	if oper_status.contains(~"(1)")
	{
		let prefix = fmt!("sname:%s-", name);
		
		let old_url = option::Some(IriValue(~"http://network/" + managed_ip));
		let old_timestamp = get_old_f64(old_url, ~"gnos:timestamp", old);
		let delta_s = uptime as f64 - old_timestamp;
		
		let (out_bps, out_bps_str) = get_per_second_value_str(interface, ~"ifOutOctets", old_url, prefix + ~"ifOutOctets", old, delta_s, ~"b");
		if  !float::is_NaN(out_bps)
		{
if name == ~"lo" && managed_ip == ~"10.103.0.2"
{
	match lookup(interface, ~"ifOutOctets", ~"")
	{
		~"" =>
		{
		}
		value =>
		{
let new_value = 8.0*(i64::from_str(value).get() as float);
let old_value = 8.0*(get_old_f64(old_url, prefix + ~"ifOutOctets", old) as float);
io::println(fmt!("secs: %.1f, delta: %.1f, bps: %.1f", delta_s as float, new_value - old_value, (new_value - old_value)/(delta_s as float)));
		}
	}
}
			add_interface_out_meter(store, managed_ip, name, interface, out_bps);
		}
		
		// TODO: We're not showing ifInUcastPkts and ifOutUcastPkts because bandwidth seems
		// more important, the table starts to get cluttered when we do, and multicast is at least as
		// important (to me anyway). I think what we should do is have a link somewhere that
		// displays a big chart allowing the client to pick which interfaces to display and which
		// traffic types (of course we'd also have to rely on either some other MIB or something
		// like Netflow).
		html += ~"<tr>\n";
			html += fmt!("<td>%s</td>", name);
			html += fmt!("<td>%s%s</td>", get_str_cell(interface, ~"ipAdEntAddr"), get_subnet(interface));
			html += fmt!("<td>%s</td>", get_per_second_str(interface, ~"ifInOctets", old_url, prefix + ~"ifInOctets", old, delta_s, ~"b"));
			html += fmt!("<td>%s</td>", out_bps_str);
			html += fmt!("<td>%s</td>", get_int_cell(interface, ~"ifSpeed", ~"bps"));
			html += fmt!("<td>%s</td>", get_str_cell(interface, ~"ifPhysAddress").to_upper());
			html += fmt!("<td>%s</td>", get_int_cell(interface, ~"ifMtu", ~"B"));
			html += fmt!("<td><a href='./subject/snmp/snmp:%s-%s'>data</a></td>", managed_ip, name);
		html += ~"\n</tr>\n";
		
		// These are undocumented because they are not intended to be used by clients.
		let entries = ~[
			(prefix + ~"ifInOctets", IntValue(get_snmp_i64(interface, ~"ifInOctets", 0))),
			(prefix + ~"ifOutOctets", IntValue(get_snmp_i64(interface, ~"ifOutOctets", 0))),
		];
		store.add(old_subject, entries);
	}
	
	toggle_interface_uptime_alert(alerts_store, managed_ip, interface, name, uptime);
	toggle_admin_vs_oper_interface_alert(alerts_store, managed_ip, interface, name, oper_status);
	toggle_weird_interface_state_alert(alerts_store, managed_ip, name, oper_status);
	
	return (name, html);
}

fn add_interface_out_meter(store: &Store, managed_ip: ~str, name: ~str, interface: std::map::hashmap<~str, std::json::Json>, out_bps: float)
{
	let if_speed = get_snmp_i64(interface, ~"ifSpeed", 0) as float;
	let level = out_bps/if_speed;
	if if_speed > 0.0 && level > 0.1
	{
		let subject = get_blank_name(store, fmt!("%s-meter", managed_ip));
		store.add(subject, ~[
			(~"gnos:meter",          StringValue(name, ~"")),
			(~"gnos:target",          IriValue(fmt!("devices:%s", managed_ip))),
			(~"gnos:level",           FloatValue(level as f64)),
			(~"gnos:description",  StringValue(~"Percentage of interface bandwidth used by output packets.", ~"")),
		]);
	}
}

fn toggle_interface_uptime_alert(alerts_store: &Store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::Json>, name: ~str, sys_uptime: float)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = name + ~"-uptime";
	let if_time = (get_snmp_i64(interface, ~"ifLastChange", -1) as float)/100.0;
	let time = sys_uptime - if_time;
	
	if if_time >= 0.0 && time < 60.0		// only signal changed if we actually got an up time
	{
		// TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
		let mesg = fmt!("%s status changed.", name);		// we can't add the time here because alerts aren't changed when re-opened (and the mesg doesn't change when they are closed)
		model::open_alert(alerts_store, {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
	}
	else
	{
		model::close_alert(alerts_store, device, id);
	}
}

fn toggle_admin_vs_oper_interface_alert(alerts_store: &Store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::Json>, name: ~str, oper_status: ~str)
{
	let admin_status = lookup(interface, ~"ifAdminStatus", ~"");
	
	let device = fmt!("devices:%s", managed_ip);
	let id = name + ~"-status";
	if admin_status.is_not_empty() && oper_status != admin_status
	{
		let mesg = fmt!("Admin set %s to %s, but operational state is %s.", name, trim_interface_status(admin_status), trim_interface_status(oper_status));
		model::open_alert(alerts_store, {device: device, id: id, level: model::ErrorLevel, mesg: mesg, resolution: ~""});
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
fn toggle_weird_interface_state_alert(alerts_store: &Store, managed_ip: ~str, name: ~str, oper_status: ~str)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = name + ~"-weird";
	if oper_status.contains(~"(1)") || oper_status.contains(~"(2)")
	{
		model::close_alert(alerts_store, device, id);
	}
	else
	{
		let mesg = fmt!("%s operational state is %s.", name, trim_interface_status(oper_status));
		model::open_alert(alerts_store, {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
	}
}

// Remove "\(\d+\)" from an interface status string.
// TODO: Should use a regex once rust supports them.
fn trim_interface_status(status: ~str) -> ~str
{
	let mut result = status;
	
	for uint::range(1, 7)
	|i|
	{
		result = str::replace(result, fmt!("(%?)", i), ~"");
	}
	
	return result;
}

fn get_subnet(interface: std::map::hashmap<~str, std::json::Json>) -> ~str
{
	match lookup(interface, ~"ipAdEntNetMask", ~"")
	{
		~"" =>
		{
			~"/?"
		}
		s =>
		{
			let parts = s.split_char('.');
			let bytes = do parts.map |p| {uint::from_str(p).get()};		// TODO: probably shouldn't fail for malformed json
			let mask = do bytes.foldl(0) |sum, current| {256*sum + current};
			let leading = count_leading_ones(mask);
			let trailing = count_trailing_zeros(mask);
			if leading + trailing == 32
			{
				fmt!("/%?", leading)
			}
			else
			{
				// Unusual netmask where 0s and 1s are mixed.
				fmt!("/%s", s)
			}
		}
	}
}

#[test]
fn test_get_subnet()
{
	let interface = std::map::str_hash();
	assert get_subnet(interface) == ~"/?";
	
	interface.insert(~"ipAdEntNetMask", std::json::String(@~"255.255.255.255"));
	assert get_subnet(interface) == ~"/32";
	
	interface.insert(~"ipAdEntNetMask", std::json::String(@~"255.0.0.0"));
	assert get_subnet(interface) == ~"/8";
	
	interface.insert(~"ipAdEntNetMask", std::json::String(@~"0.0.0.0"));
	assert get_subnet(interface) == ~"/0";
	
	interface.insert(~"ipAdEntNetMask", std::json::String(@~"255.0.1.0"));
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
	
	return count;
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
	
	return count;
}

fn get_per_second_str(data: std::map::hashmap<~str, std::json::Json>, key: ~str, old_url: option::Option<Object>, name: ~str, old: &Solution, delta_s: f64, unit: ~str) -> ~str
{
	match lookup(data, key, ~"")
	{
		~"" =>
		{
			~"\n"
		}
		value =>
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
				fmt!("%s%sps\n", pps_str_value, unit)
			}
			else
			{
				fmt!("%s%s\n", new_str_value, unit)
			}
		}
	}
}

fn get_per_second_value_str(data: std::map::hashmap<~str, std::json::Json>, key: ~str, old_url: option::Option<Object>, name: ~str, old: &Solution, delta_s: f64, unit: ~str) -> (float, ~str)
{
	match lookup(data, key, ~"")
	{
		~"" =>
		{
			(float::NaN, ~"\n")
		}
		value =>
		{
			let new_value = 8.0f64*(i64::from_str(value).get() as f64);
			let new_str_value = utils::f64_to_unit_str(new_value);
			
			let old_value = 8.0f64*get_old_f64(old_url, name, old);
			if old_value > 0.0f64 && delta_s > 1.0f64
			{
				// Showing the absolute packet numbers is nearly useless so we'll only
				// show packets per second if it is available.
				let pps = (new_value - old_value)/delta_s;
				let pps_str_value = utils::f64_to_unit_str(pps);
				(pps as float, fmt!("%s%sps\n", pps_str_value, unit))
			}
			else
			{
				(float::NaN, fmt!("%s%s\n", new_str_value, unit))
			}
		}
	}
}

fn get_old_f64(old_url: option::Option<Object>, predicate: ~str, old: &Solution) -> f64
{
	let old_row = old.rows.find(|r| {r.search(~"subject") == old_url && r.search(~"name") == option::Some(StringValue(predicate, ~""))});
	if old_row.is_some()
	{
		old_row.get().get(~"value").as_f64()
	}
	else
	{
		0.0f64
	}
}

fn get_scaled_int_value(data: std::map::hashmap<~str, std::json::Json>, label: ~str, key: ~str, units: ~str, scaling: i64) -> ~str
{
	let value = get_snmp_i64(data, key, 0)*scaling;
	if value > 0
	{
		fmt!("<strong>%s:</strong> %s%s<br>\n", label, utils::i64_to_unit_str(value), units)
	}
	else
	{
		~""
	}
}

fn get_int_cell(data: std::map::hashmap<~str, std::json::Json>, key: ~str, units: ~str) -> ~str
{
	let value = get_snmp_i64(data, key, 0);
	if value > 0
	{
		fmt!("%s%s", utils::i64_to_unit_str(value), units)
	}
	else
	{
		~""
	}
}

fn get_str_cell(data: std::map::hashmap<~str, std::json::Json>, key: ~str) -> ~str
{
	lookup(data, key, ~"")
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn json_to_snmp(remote_addr: ~str, store: &Store, data: std::map::hashmap<~str, std::json::Json>)
{
	store.clear();
	
	for data.each
	|key, value|
	{
		match value
		{
			std::json::Dict(d) =>
			{
				device_to_snmp(remote_addr, store, key, d);
			}
			_ =>
			{
				error!("%s was expected to have a device map but %s was %?", remote_addr, key, value);
			}
		}
	}
}

fn device_to_snmp(remote_addr: ~str, store: &Store, managed_ip: ~str, data: std::map::hashmap<~str, std::json::Json>)
{
	let mut entries = ~[];
	vec::reserve(entries, data.size());
	
	for data.each		// unfortunately hashmap doesn't support the base_iter protocol so there's no nice way to do this
	|name, value|
	{
		match value
		{
			std::json::String(s) =>
			{
				vec::push(entries, (~"sname:" + name, StringValue(*s, ~"")));
			}
			std::json::List(interfaces) =>
			{
				interfaces_to_snmp(remote_addr, store, managed_ip, interfaces);
			}
			_ =>
			{
				error!("%s device was expected to contain string or list but %s was %?", remote_addr, name, value);
			}
		}
	};
	
	let subject = fmt!("snmp:%s", managed_ip);
	store.add(subject, entries);
}

fn interfaces_to_snmp(remote_addr: ~str, store: &Store, managed_ip: ~str, interfaces: @~[std::json::Json])
{
	for interfaces.each
	|data|
	{
		match data
		{
			std::json::Dict(interface) =>
			{
				interface_to_snmp(remote_addr, store, managed_ip, interface);
			}
			_ =>
			{
				error!("%s interfaces was expected to contain string or list but found %?", remote_addr, data);
			}
		}
	}
}

fn interface_to_snmp(remote_addr: ~str, store: &Store, managed_ip: ~str, interface: std::map::hashmap<~str, std::json::Json>)
{
	let mut ifname = ~"";
	let mut entries = ~[];
	vec::reserve(entries, interface.size());
	
	for interface.each
	|name, value|
	{
		match value
		{
			std::json::String(s) =>
			{
				if name == ~"ifDescr"
				{
					ifname = *s;
				}
				vec::push(entries, (~"sname:" + name, StringValue(*s, ~"")));
			}
			_ =>
			{
				error!("%s interfaces was expected to contain a string or dict but %s was %?", remote_addr, name, value);
			}
		}
	};
	
	if ifname.is_not_empty()
	{
		let subject = fmt!("snmp:%s", managed_ip + "-" + ifname);
		store.add(subject, entries);
	}
	else
	{
		error!("%s interface was missing an ifDescr:", remote_addr);
		for interface.each() |k, v| {error!("   %s => %?", k, v);};
	}
}

fn get_snmp_i64(table: std::map::hashmap<~str, std::json::Json>, key: ~str, default: i64) -> i64
{
	match lookup(table, key, ~"")
	{
		~"" =>
		{
			default
		}
		text =>
		{
			match i64::from_str(text)
			{
				option::Some(value) =>
				{
					value
				}
				option::None =>
				{
					error!("%s was %s, but expected an int", key, text);
					default
				}
			}
		}
	}
}

// Lookup an SNMP value.
fn lookup(table: std::map::hashmap<~str, std::json::Json>, key: ~str, default: ~str) -> ~str
{
	match table.find(key)
	{
		option::Some(std::json::String(s)) =>
		{
			*s
		}
		option::Some(value) =>
		{
			// This is something that should never happen so it's not so bad that we don't provide a lot of context
			// (if it does somehow happen admins can crank up the logging level to see where it is coming from).
			error!("%s was expected to be a string but is a %?", key, value);	// TODO: would be nice if the site could somehow show logs
			default
		}
		option::None =>
		{
			default
		}
	}
}

