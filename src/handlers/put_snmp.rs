/// This is the code that handles PUTs from the snmp-modeler script. It parses the
/// incoming json, converts it into triplets, and updates the model.
use core::dvec::*;
use std::json::{Json};
use json = std::json;
use std::map::*;
use model::{Msg, UpdateMsg, UpdatesMsg, QueryMsg, eval_query};
use options::{Options, Device};
use rrdf::rrdf::*;
use runits::generated::*;
use runits::units::*;
use snmp::*;
use task_runner::*;
use comm::{Chan, Port};
use server = rwebserve::rwebserve;
use mustache::*;
use mustache::{Template, TemplateTrait};
use core::io::{WriterUtil};
use mustache::{Context, ContextTrait};

type SamplesChan = Chan<samples::Msg>;

// This is equivalent to an hours worth of data at a fast poll rate (20s). Slower poll rates (which
// are expected to me more likely) will retain correspondingly longer time spans.
const samples_capacity: uint = 180;

struct Network
{
	options: &Options,
	samples_chan: SamplesChan,
	remote_addr: ~str,
	store: &Store,
	alerts_store: &Store,
	snmp_store: &Store,
	snmp: HashMap<~str, Json>,
}

fn put_snmp(options: &Options, state_chan: Chan<Msg>, samples_chan: SamplesChan, request: &server::Request, response: &server::Response) -> server::Response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	let addr = copy request.remote_addr;
	info!("got new modeler data from %s", addr);
	
	// Arguably cleaner to do this inside of json_to_store (or add_device) but we'll deadlock if we try
	// to do a query inside of an updates_mesg callback.
	let old = query_old_info(state_chan);
	let options = copy *options;
	comm::send(state_chan, UpdatesMsg(~[~"primary", ~"snmp", ~"alerts"], |ss, d, move options| {updates_snmp(&options, samples_chan, addr, ss, d, &old)}, copy request.body));
	
	server::Response {body: rwebserve::configuration::StringBody(@~""), ..*response}
}

priv fn updates_snmp(options: &Options, samples_chan: SamplesChan, remote_addr: ~str, stores: &[@Store], body: ~str, old: &Solution) -> bool
{
	match json::from_str(body)
	{
		result::Ok(data) =>
		{
			match data
			{
				json::Dict(d) =>
				{
					let network = Network
					{
						options: options,
						samples_chan: samples_chan,
						remote_addr: remote_addr,
						store: &*stores[0],
						alerts_store: &*stores[2],
						snmp_store: &*stores[1],
						snmp: d,
					};
					
					json_to_primary(&network, d, old);
					json_to_snmp(&network);
					samples_chan.send(samples::SyncMsg);
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

priv fn query_old_info(state_chan: Chan<Msg>) -> Solution
{
	let po = Port();
	let ch = Chan(po);
	
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
priv fn get_sparkline_script(options: &Options) -> Path
{
	let path = options.root.pop();				// gnos
	let path = path.push(~"scripts");
	let path = path.push(~"sparkline.R");		// gnos/scripts/sparkline.R
	path
}

priv fn json_to_primary(network: &Network, data: HashMap<~str, Json>, old: &Solution)
{
	network.store.clear();
	network.store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:last_update", object: DateTimeValue(std::time::now())});
	network.store.add_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:poll_interval", object: IntValue(network.options.poll_rate as i64)});
	
	let path = get_sparkline_script(network.options);
	let context = mustache::context(@~".", @~"");
	let template = context.compile_file(path.to_str());
	
	let mut charts = ~[];
	let mut script = ~"";
	for data.each() |managed_ip, the_device|
	{
		match the_device
		{
			json::Dict(device) =>
			{
				let old_subject = get_blank_name(network.store, ~"old");
				add_device(network, managed_ip, device, old, old_subject, template, &mut script, &mut charts);
				add_device_notes(network, managed_ip, device);
			}
			_ =>
			{
				error!("%s device from %s was expected to be a dict but is a %?", managed_ip, network.remote_addr, the_device);	// TODO: probably want to add errors to store
			}
		}
	};
	
	if charts.is_not_empty()
	{
		// We always create these charts. That's a bit wasteful because they don't appear on the main page.
		// However building an URL that encodes all the info neccesary to create them would be rather
		// awful. TODO: I guess samples could store a Chart struct and then use that to dynamically create
		// the charts.
		samples::create_charts(~"devices", charts, network.samples_chan);
	}
	if script.is_not_empty()
	{
		run_r_script(script);
	}
	
	info!("Received data from %s:", network.remote_addr);
	//for store.each |triple| {info!("   %s", triple.to_str());};
}

priv fn run_r_script(script: ~str)
{
	fn get_output(label: &str, reader: io::Reader) -> ~str
	{
		let text = str::from_bytes(reader.read_whole_stream());
		if text.is_not_empty() {fmt!("%s:\n%s\n", label, text)} else {~""}
	}
	
	let script = ~"library(YaleToolkit)\n\n" + script;
	let action: JobFn = 
		||
		{
			let path = path::from_str("/tmp/gnos-sparkline.R");		// TODO use a better path once rust has a better tmp file function
			match io::file_writer(&path, ~[io::Create, io::Truncate])
			{
				result::Ok(writer) =>
				{
					writer.write_str(script);
					
					let program = core::run::start_program("Rscript", [path.to_str()]);
					let result = program.finish();
					if result != 0
					{
						let mut err = fmt!("Rscript %s returned %?\n", path.to_str(), result);
						err += get_output("stdout", program.output());
						err += get_output("stderr", program.err());
						option::Some(err)
					}
					else
					{
						option::None
					}
				}
				result::Err(ref err) =>
				{
					option::Some(fmt!("Failed to create %s: %s", path.to_str(), *err))
				}
			}
		};
	let cleanup: ExitFn = || {};
	run(Job {action: action, policy: IgnoreFailures}, ~[cleanup]);
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
priv fn add_device(network: &Network, managed_ip: ~str, device: HashMap<~str, Json>, old: &Solution, old_subject: ~str, template: mustache::Template, script: &mut ~str, charts: &mut ~[samples::Chart])
{
	match network.options.devices.find(|d| {d.managed_ip == managed_ip})
	{
		option::Some(ref options_device) =>
		{
			let old_url = option::Some(IriValue(~"http://network/" + managed_ip));
			let snmp = Snmp(device, device, copy *old,  ~"sname:", old_url);
			let time = snmp.new_time;
			
			let entries = ~[
				(~"gnos:center_x", FloatValue(options_device.center_x as f64)),
				(~"gnos:center_y", FloatValue(options_device.center_y as f64)),
				(~"gnos:style", StringValue(copy options_device.style, ~"")),
				
				(~"gnos:primary_label", StringValue(copy options_device.name, ~"")),
				(~"gnos:secondary_label", StringValue(copy managed_ip, ~"")),
				(~"gnos:tertiary_label", StringValue(get_device_label(&snmp).trim(), ~"")),
			];
			let subject = fmt!("devices:%s", managed_ip);
			network.store.add(subject, entries);
			
			// These are undocumented because they not intended to be used by clients.
			network.store.add_triple(~[], {subject: subject, predicate: ~"gnos:internal-info", object: BlankValue(copy old_subject)});
			
			let entries = [
				(~"gnos:timestamp", option::Some(snmp.new_time)),
				(~"sname:ipInReceives", snmp.get_value(~"ipInReceives", Packet)),
				(~"sname:ipForwDatagrams", snmp.get_value(~"ipForwDatagrams", Packet)),
				(~"sname:ipInDelivers", snmp.get_value(~"ipInDelivers", Packet)),
			];
			add_value_entries(network.store, old_subject, entries);
			
			toggle_device_uptime_alert(network.alerts_store, managed_ip, time);
			
			let interfaces = device.find(~"interfaces");
			if interfaces.is_some()
			{
				let has_interfaces = add_interfaces(network, device, managed_ip, interfaces.get(), old, old_subject, time, template, script, charts);
				toggle_device_down_alert(network.alerts_store, managed_ip, has_interfaces);
			}
			else
			{
				toggle_device_down_alert(network.alerts_store, managed_ip, false);
			}
		}
		option::None =>
		{
			error!("Couldn't find %s in the network json file", managed_ip);
		}
	};
}

priv fn toggle_device_uptime_alert(alerts_store: &Store, managed_ip: ~str, time: Value)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = ~"uptime";
	
	if time.value >= 0.0 && time < from_units(60.0, Second)		// only reboot if we actually got an up time
	{
		// TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
		let mesg = ~"Device rebooted.";		// we can't add the time here because alerts aren't changed when re-opened (and the mesg doesn't change when they are closed)
		model::open_alert(alerts_store, &model::Alert {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
	}
	else
	{
		model::close_alert(alerts_store, device, id);
	}
}

priv fn toggle_device_down_alert(alerts_store: &Store, managed_ip: ~str, up: bool)
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
		model::open_alert(alerts_store, &model::Alert {device: device, id: id, level: model::ErrorLevel, mesg: mesg, resolution: resolution});
	}
}

priv fn add_device_notes(network: &Network, managed_ip: ~str, _device: HashMap<~str, Json>)
{
	// summary
	let html = #fmt["
<p class='summary'>
	The name and ip address are from the network json file. All the other info is from <a href='./subject/snmp/snmp:%s'>SNMP</a>.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipInReceives&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Received </a> is the number of packets received on interfaces.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipForwDatagrams&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Forwarded </a> is the number of packets received but not destined for the device.
	<a href='http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=ipInDelivers&translate=Translate&submitValue=SUBMIT&submitClicked=true'>Delivered </a> is the number of packets sent to a local IP protocol.
</p>", managed_ip];
	
	let subject = get_blank_name(network.store, ~"summary");
	network.store.add(subject, ~[
		(~"gnos:title",       StringValue(~"notes", ~"")),
		(~"gnos:target",    IriValue(fmt!("devices:%s", managed_ip))),
		(~"gnos:detail",    StringValue(html, ~"")),
		(~"gnos:weight",  FloatValue(0.1f64)),
		(~"gnos:open",     StringValue(~"no", ~"")),
		(~"gnos:key",       StringValue(~"device notes", ~"")),
	]);
}

priv fn get_device_label(snmp: &Snmp) -> ~str
{
	let mut label = ~"";
	
	let x = snmp.get_value_per_sec(~"ipInReceives", Packet);
	if x.is_some() {label += fmt!("recv: %s\n", get_si_str(x, "%.1f"));}
	
	let x = snmp.get_value_per_sec(~"ipForwDatagrams", Packet);
	if x.is_some() {label += fmt!("fwd: %s\n", get_si_str(x, "%.1f"));}
	
	let x = snmp.get_value_per_sec(~"ipInDelivers", Packet);
	if x.is_some() {label += fmt!("del: %s\n", get_si_str(x, "%.1f"));}
	
	label
}

priv fn add_interfaces(network: &Network, device: HashMap<~str, Json>, managed_ip: ~str, data: Json, old: &Solution, old_subject: ~str, uptime: Value, template: mustache::Template, script: &mut ~str, charts: &mut ~[samples::Chart]) -> bool
{
	let mut in_samples = ~[];		// [(sample name, legend)]
	let mut out_samples = ~[];	// ditto
	
	let has_interfaces = match data
	{
		json::List(interfaces) =>
		{
			let mut rows = ~[];			// [(ifname, html)]
			for interfaces.each |interface|
			{
				match *interface
				{
					json::Dict(d) =>
					{
						let (name, html, in_sample, out_sample) = add_interface(network, managed_ip, device, d, old, old_subject, uptime, template, script);
						vec::push(rows, (copy name, html));
						if in_sample.is_not_empty()   {vec::push(in_samples, (in_sample, copy name))}
						if out_sample.is_not_empty() {vec::push(out_samples, (out_sample, copy name))}
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
					html += ~"<th>In (kbps)</th>\n";
					html += ~"<th>Out (kbps)</th>\n";
					html += ~"<th>Speed</th>\n";
					html += ~"<th>MAC Address</th>\n";
					html += ~"<th>MTU</th>\n";
					html += ~"<th>SNMP</th>\n";
				html += ~"</tr>\n";
				html += str::connect(hrows, ~"\n");
			html += ~"</table>\n";
			html += ~"<p class='note'>The shaded area in the sparklines is the inter-quartile range: the bounds within which half the samples fall.</p>";
			
			let subject = get_blank_name(network.store, ~"interfaces");
			network.store.add(subject, ~[
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
	};
	
	let path = core::os::make_absolute(&network.options.root);
	let path = path.push("generated");
	if in_samples.is_not_empty()
	{
		let in_samples = std::sort::merge_sort(|x, y| {x.second() <= y.second()}, in_samples);;
		
		let path = path.push(fmt!("%s-in-interfaces.png", managed_ip));
		let in_chart = samples::Chart
		{
			path: path.to_str(),
			sample_sets: do in_samples.map |s| {s.first()},
			legends: do in_samples.map |s| {s.second()},
			interval: network.options.poll_rate as float,
			units: Kilo*Bit/Second,
			title: fmt!("%s In Bandwidth", managed_ip),
			y_label: ~"Bandwidth",
		};
		vec::push(*charts, in_chart);
	}
	if out_samples.is_not_empty()
	{
		let out_samples = std::sort::merge_sort(|x, y| {x.second() <= y.second()}, out_samples);;
		
		let path = path.push(fmt!("%s-out-interfaces.png", managed_ip));
		let out_chart = samples::Chart
		{
			path: path.to_str(),
			sample_sets: do out_samples.map |s| {s.first()},
			legends: do out_samples.map |s| {s.second()},
			interval: network.options.poll_rate as float,
			units: Kilo*Bit/Second,
			title: fmt!("%s Out Bandwidth", managed_ip),
			y_label: ~"Bandwidth",
		};
		vec::push(*charts, out_chart);
	}
	has_interfaces
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
priv fn add_interface(network: &Network, managed_ip: ~str, device: HashMap<~str, Json>, interface: HashMap<~str, Json>, old: &Solution, old_subject: ~str, uptime: Value, template: mustache::Template, script: &mut ~str) -> (~str, ~str, ~str, ~str)
{
	let name = lookup(interface, ~"ifDescr", ~"eth?");
	let mut html = ~"";
	let mut in_sample = ~"";
	let mut out_sample = ~"";
	
	let old_url = option::Some(IriValue(~"http://network/" + managed_ip));
	let snmp = Snmp(device, interface, copy *old,  fmt!("sname:%s-", name), old_url);
	
	let oper_status = lookup(interface, ~"ifOperStatus", ~"missing");
	if oper_status.contains(~"(1)")
	{
		let prefix = fmt!("sname:%s-", name);
		
		let out_octets = snmp.get_value_per_sec(~"ifOutOctets", Byte);
		let out_octets = convert_per_sec(out_octets, Kilo*Bit);
		let sample_name = fmt!("%s-%s-out-octets", managed_ip, name);
		let out_octets_html = make_samples_html(network, out_octets, sample_name, template, script, &mut in_sample, managed_ip, "out");
		if  out_octets.is_some() && is_compound(out_octets.get())
		{
			add_interface_out_meter(network.store, &snmp, managed_ip, name, out_octets.get());
		}
		
		let in_octets = snmp.get_value_per_sec(~"ifInOctets", Byte);
		let in_octets = convert_per_sec(in_octets, Kilo*Bit);
		let sample_name = fmt!("%s-%s-in-octets", managed_ip, name);
		let in_octets_html = make_samples_html(network, in_octets, sample_name, template, script, &mut out_sample, managed_ip, "in");
		
		// TODO: We're not showing ifInUcastPkts and ifOutUcastPkts because bandwidth seems
		// more important, the table starts to get cluttered when we do, and multicast is at least as
		// important (to me anyway). I think what we should do is have a link somewhere that
		// displays a big chart allowing the client to pick which interfaces to display and which
		// traffic types (of course we'd also have to rely on either some other MIB or something
		// like Netflow).
		html += ~"<tr>\n";
			html += fmt!("<td>%s</td>", name);
			html += fmt!("<td>%s%s</td>", lookup(interface, ~"ipAdEntAddr", ~""), get_subnet(interface));
			html += fmt!("<td>%s</td>", in_octets_html);
			html += fmt!("<td>%s</td>", out_octets_html);
			html += fmt!("<td>%s</td>", 	get_si_str(snmp.get_value(~"ifSpeed", Bit/Second), "%.0f"));
			html += fmt!("<td>%s</td>", lookup(interface, ~"ifPhysAddress", ~""));
			html += fmt!("<td>%s</td>", get_value_str(snmp.get_value(~"ifMtu", Byte), "%.0f"));
			html += fmt!("<td><a href='./subject/snmp/snmp:%s-%s'>data</a></td>", managed_ip, name);
		html += ~"\n</tr>\n";
		
		// These are undocumented because they are not intended to be used by clients.
		let entries = [
			(prefix + ~"ifInOctets", snmp.get_value(~"ifInOctets", Byte)),
			(prefix + ~"ifOutOctets", snmp.get_value(~"ifOutOctets", Byte)),
		];
		add_value_entries(network.store, old_subject, entries);
	}
	
	toggle_interface_uptime_alert(network.alerts_store, managed_ip, &snmp, name, uptime);
	toggle_admin_vs_oper_interface_alert(network.alerts_store, managed_ip, interface, name, oper_status);
	toggle_weird_interface_state_alert(network.alerts_store, managed_ip, name, oper_status);
	
	return (name, html, in_sample, out_sample);
}

// TODO: argument lists are getting out of hand, probably want to introduce a struct or two
priv fn make_samples_html(network: &Network, sample: option::Option<Value>, name: ~str, template: mustache::Template, script: &mut ~str, sample_name: &mut ~str, managed_ip: ~str, direction: &str) -> ~str
{
	if  sample.is_some() && sample.get().units == Kilo*Bit/Second
	{
		let owner = fmt!("%s-%s", managed_ip, direction);
		network.samples_chan.send(samples::AddSample(owner, copy name, sample.get().value, samples_capacity));
		let (sub_script, num_adds) = build_sparkline(network, name, template);
		if sub_script.is_not_empty()
		{
			// The home page generates dynamic html and assigns it to innerHTML. Unfortunately in
			// this case the browser won't reload images, even if they have expired. So we add this silly
			// # argument which will be ignored by the server. See http://www.post-hipster.com/2008/10/20/using-javascript-to-refresh-an-image-without-a-cache-busting-parameter/
			// TODO: is there a better way?
			let url = fmt!("generated/%s.png#%?", name, num_adds);
			
			*sample_name = copy name;
			*script += sub_script;
			fmt!("<a href='interfaces/%s/%s'><img src = '%s' alt = 'octets'></a>", managed_ip, direction, url)
		}
		else
		{
			fmt!("%.2f kbps", sample.get().value)
		}
	}
	else if  sample.is_some() && sample.get().units == Kilo*Bit
	{
		// The very first sample is in kb because we need two samples to compute an average over time.
		// These are the wrong units for our sample set so we don't want to send them to samples_chan.
		fmt!("%.2f kb", sample.get().value)
	}
	else
	{
		assert sample.is_none();
		~"missing"
	}
}

// Creates an R script which when run will produce a sparkline chart for the named sample set.
priv fn build_sparkline(network: &Network, name: ~str, template: Template) -> (~str, uint)
{
	let port = Port();
	let chan = Chan(port);
	network.samples_chan.send(samples::GetSampleSet(copy name, chan));
	let (buffer, num_adds) = port.recv();
	
	if (buffer.len() > 1)
	{
		let mut path = core::os::make_absolute(&network.options.root);
		path = path.push("generated");
		path = path.push(fmt!("%s.png", name));
		
		let context = HashMap();
		context.insert(@~"samples", mustache::Str(@str::connect(do iter::map_to_vec(buffer) |s| {s.to_str()}, ", ")));
		context.insert(@~"file", mustache::Str(@path.to_str()));
		context.insert(@~"width", mustache::Str(@~"150"));
		context.insert(@~"height", mustache::Str(@~"50"));
		context.insert(@~"label", mustache::Str(@~"kbps"));
		
		(template.render(context), num_adds)
	}
	else
	{
		(~"", num_adds)
	}
}

priv fn add_interface_out_meter(store: &Store, snmp: &Snmp, managed_ip: ~str, name: ~str, out_bps: Value)
{
	let if_speed = snmp.get_value(~"ifSpeed", Bit/Second);
	if if_speed.is_some()
	{
		let level = out_bps/if_speed.get();
		if level.value > 0.1
		{
			let subject = get_blank_name(store, fmt!("%s-meter", managed_ip));
			store.add(subject, ~[
				(~"gnos:meter",          StringValue(copy name, ~"")),
				(~"gnos:target",          IriValue(fmt!("devices:%s", managed_ip))),
				(~"gnos:level",           FloatValue(level.value as f64)),
				(~"gnos:description",  StringValue(~"Percentage of interface bandwidth used by output packets.", ~"")),
			]);
		}
	}
}

priv fn toggle_interface_uptime_alert(alerts_store: &Store, managed_ip: ~str, snmp: &Snmp, name: ~str, sys_uptime: Value)
{
	let device = fmt!("devices:%s", managed_ip);
	let id = name + ~"-uptime";
	let if_time = snmp.get_value(~"ifLastChange", Centi*Second);
	if if_time.is_some()
	{
		let if_time = if_time.get().convert_to(Second);
		let time = sys_uptime - if_time;
		
		if time.value < 60.0
		{
			// TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
			let mesg = fmt!("%s status changed.", name);		// we can't add the time here because alerts aren't changed when re-opened (and the mesg doesn't change when they are closed)
			model::open_alert(alerts_store, &model::Alert {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
		}
		else
		{
			model::close_alert(alerts_store, device, id);
		}
	}
}

priv fn toggle_admin_vs_oper_interface_alert(alerts_store: &Store, managed_ip: ~str, interface: HashMap<~str, Json>, name: ~str, oper_status: ~str)
{
	let admin_status = lookup(interface, ~"ifAdminStatus", ~"");
	
	let device = fmt!("devices:%s", managed_ip);
	let id = name + ~"-status";
	if admin_status.is_not_empty() && oper_status != admin_status
	{
		let mesg = fmt!("Admin set %s to %s, but operational state is %s.", name, trim_interface_status(admin_status), trim_interface_status(oper_status));
		model::open_alert(alerts_store, &model::Alert {device: device, id: id, level: model::ErrorLevel, mesg: mesg, resolution: ~""});
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
priv fn toggle_weird_interface_state_alert(alerts_store: &Store, managed_ip: ~str, name: ~str, oper_status: ~str)
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
		model::open_alert(alerts_store, &model::Alert {device: device, id: id, level: model::WarningLevel, mesg: mesg, resolution: ~""});
	}
}

// Remove "\(\d+\)" from an interface status string.
// TODO: Should use a regex once rust supports them.
priv fn trim_interface_status(status: ~str) -> ~str
{
	let mut result = copy status;
	
	for uint::range(1, 7)
	|i|
	{
		result = str::replace(result, fmt!("(%?)", i), ~"");
	}
	
	return result;
}

priv fn is_compound(x: Value) -> bool
{
	match x.units
	{
		Compound(*)	=> true,
		_				=> false,
	}
}

priv fn get_subnet(interface: HashMap<~str, Json>) -> ~str
{
	match lookup(interface, ~"ipAdEntNetMask", ~"")
	{
		~"" =>
		{
			~"/?"
		}
		ref s =>
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
				fmt!("/%s", *s)
			}
		}
	}
}

#[test]
fn test_get_subnet()
{
	let interface = HashMap();
	assert get_subnet(interface) == ~"/?";
	
	interface.insert(~"ipAdEntNetMask", json::String(@~"255.255.255.255"));
	assert get_subnet(interface) == ~"/32";
	
	interface.insert(~"ipAdEntNetMask", json::String(@~"255.0.0.0"));
	assert get_subnet(interface) == ~"/8";
	
	interface.insert(~"ipAdEntNetMask", json::String(@~"0.0.0.0"));
	assert get_subnet(interface) == ~"/0";
	
	interface.insert(~"ipAdEntNetMask", json::String(@~"255.0.1.0"));
	assert get_subnet(interface) == ~"/255.0.1.0";
}

priv fn count_leading_ones(mask: uint) -> int
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

priv fn count_trailing_zeros(mask: uint) -> int
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

priv fn add_value_entries(store: &Store, subject: ~str, entries: &[(~str, option::Option<Value>)])
{
	let entries = do entries.filter_map
	|e|
	{
		if e.second().is_some()
		{
			option::Some((e.first(), FloatValue(e.second().get().value as f64)))
		}
		else
		{
			option::None
		}
	};
	store.add(subject, entries);
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
priv fn json_to_snmp(network: &Network)
{
	network.snmp_store.clear();
	
	for network.snmp.each
	|key, value|
	{
		match value
		{
			json::Dict(d) =>
			{
				device_to_snmp(network, key, d);
			}
			_ =>
			{
				error!("%s was expected to have a device map but %s was %?", network.remote_addr, key, value);
			}
		}
	}
}

priv fn device_to_snmp(network: &Network, managed_ip: ~str, data: HashMap<~str, Json>)
{
	let mut entries = ~[];
	vec::reserve(entries, data.size());
	
	for data.each		// unfortunately HashMap doesn't support the base_iter protocol so there's no nice way to do this
	|name, value|
	{
		match value
		{
			json::String(s) =>
			{
				vec::push(entries, (~"sname:" + name, StringValue(copy *s, ~"")));
			}
			json::List(interfaces) =>
			{
				interfaces_to_snmp(network, managed_ip, interfaces);
			}
			_ =>
			{
				error!("%s device was expected to contain string or list but %s was %?", network.remote_addr, name, value);
			}
		}
	};
	
	let subject = fmt!("snmp:%s", managed_ip);
	network.snmp_store.add(subject, entries);
}

priv fn interfaces_to_snmp(network: &Network, managed_ip: ~str, interfaces: @~[Json])
{
	for interfaces.each
	|data|
	{
		match *data
		{
			json::Dict(interface) =>
			{
				interface_to_snmp(network, managed_ip, interface);
			}
			_ =>
			{
				error!("%s interfaces was expected to contain string or list but found %?", network.remote_addr, data);
			}
		}
	}
}

priv fn interface_to_snmp(network: &Network, managed_ip: ~str, interface: HashMap<~str, Json>)
{
	let mut ifname = ~"";
	let mut entries = ~[];
	vec::reserve(entries, interface.size());
	
	for interface.each
	|name, value|
	{
		match value
		{
			json::String(s) =>
			{
				if name == ~"ifDescr"
				{
					ifname = copy *s;
				}
				vec::push(entries, (~"sname:" + name, StringValue(copy *s, ~"")));
			}
			_ =>
			{
				error!("%s interfaces was expected to contain a string or dict but %s was %?", network.remote_addr, name, value);
			}
		}
	};
	
	if ifname.is_not_empty()
	{
		let subject = fmt!("snmp:%s", managed_ip + "-" + ifname);
		network.snmp_store.add(subject, entries);
	}
	else
	{
		error!("%s interface was missing an ifDescr:", network.remote_addr);
		for interface.each() |k, v| {error!("   %s => %?", k, v);};
	}
}

priv fn get_value_str(value: option::Option<Value>, format: &str) -> ~str
{
	if value.is_some()
	{
		let ustr = value.get().units.to_str();
		let ustr = str::replace(ustr, ~"b/s", ~"bps");
		let ustr = str::replace(ustr, ~"p/s", ~"pps");
		match format.to_unique()									// matching is awfully lame, but fmt! requires a string literal and there doesn't appear to be a good alternative
		{
			~"%.0f"	=> fmt!("%.0f %s", value.get().value, ustr),	// TODO: use to_unique to avoid an ICE
			~"%.1f"	=> fmt!("%.1f %s", value.get().value, ustr),
			~"%.2f"	=> fmt!("%.2f %s", value.get().value, ustr),
			~"%.3f"	=> fmt!("%.3f %s", value.get().value, ustr),
			_			=> fail ~"bad format string: " + format,
		}
	}
	else
	{
		~"?"
	}
}

priv fn get_si_str(value: option::Option<Value>, format: &str) -> ~str
{
	if value.is_some()
	{
		let value = option::Some(value.get().normalize_si());
		get_value_str(value, format)
	}
	else
	{
		~"?"
	}
}

priv fn convert_per_sec(x: option::Option<Value>, to: Unit) -> option::Option<Value>
{
	do x.chain
	|value|
	{
		if is_compound(value)
		{
			option::Some(value.convert_to(to/Second))
		}
		else
		{
			option::Some(value.convert_to(to))
		}
	}
}
