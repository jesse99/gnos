/// This is the code that handles PUTs from the snmp-modeler script. It parses the
/// incoming json, converts it into triplets, and updates the model.
use core::io::{WriterUtil, ReaderUtil};
use std::json::{Json};
use json = std::json;
use std::map::*;
//use send_map::linear::*;
use model::{Msg, UpdateMsg, UpdatesMsg, QueryMsg, eval_query};
use options::{Options, Device};
use rrdf::rrdf::*;
//use runits::generated::*;
//use runits::units::*;
//use snmp::*;
use task_runner::*;
use comm::{Chan, Port};
use server = rwebserve::rwebserve;
use mustache::{Context, Template};

pub type SamplesChan = Chan<samples::Msg>;

// This is equivalent to an hours worth of data at a fast poll rate (20s). Slower poll rates (which
// are expected to be more likely) will retain correspondingly longer time spans.
pub const samples_capacity: uint = 180;

pub fn put_snmp(options: &Options, state_chan: Chan<Msg>, samples_chan: SamplesChan, request: &server::Request, response: &server::Response) -> server::Response
{
	// Unfortunately we don't send an error back to the modeler if the json was invalid.
	// Of course that shouldn't happen...
	let addr = copy request.remote_addr;
	info!("got new modeler data from %s", addr);
	
	let options = copy *options;
	comm::send(state_chan, UpdateMsg(~"primary", |s, d, move options| {handle_update(&options, addr, s, d, samples_chan)}, copy request.body));
	
	server::Response {body: rwebserve::configuration::StringBody(@~""), ..*response}
}

priv fn handle_update(options: &Options, remote_addr: &str, store: &Store, body: &str, samples_chan: SamplesChan) -> bool
{
	match json::from_str(body)
	{
		result::Ok(ref data) =>
		{
			match *data
			{
				json::Object(ref d) =>
				{
					store.replace_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:last_update", object: DateTimeValue(std::time::now())});
					store.replace_triple(~[], {subject: ~"gnos:map", predicate: ~"gnos:poll_interval", object: IntValue(options.poll_rate as i64)});
					
					let mut modeler = option::None;
					if d.contains_key(&~"modeler")
					{
						modeler = prune_modeler(store, d.get_ref(&~"modeler"));
					}
					do optional_list(data, ~"entities") |list| {add_entities(store, &modeler, list);};
					do optional_list(data, ~"labels") |list| {add_labels(store, &modeler, list);};
					do optional_list(data, ~"gauges") |list| {add_gauges(store, &modeler, list);};
					do optional_list(data, ~"details") |list| {add_details(store, &modeler, list);};
					do optional_list(data, ~"relations") |list| {add_relations(store, &modeler, list);};
					do optional_list(data, ~"alerts") |list| {add_alerts(store, list);};
					do optional_list(data, ~"samples") |list| {add_samples(options, samples_chan, list);};
					do optional_list(data, ~"charts") |list| {add_charts(options, samples_chan, list);};
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

priv fn add_entities(store: &Store, modeler: &Option<Object>, list: &json::List)
{
	fn add_entity(store: &Store, modeler: &Option<Object>, object: &Json)
	{
		let mut entries = ~[];
		if modeler.is_some()
		{
			entries.push((~"gnos:modeler-subject", modeler.get()));
		}
		entries.push((~"gnos:entity", StringValue(get_str(object, ~"label"), ~"")));
		do optional_str(object, ~"style") |value| 		{entries.push((~"gnos:style", StringValue(value, ~"")))};
		do optional_str(object, ~"predicate") |value|	{entries.push((~"gnos:predicate", StringValue(value, ~"")))};
		
		let subject = ~"entities:" + get_str(object, ~"id");
		store.add(subject, entries);
	}
	
	for list.each |entity|
	{
		add_entity(store, modeler, entity);
	}
}

priv fn add_labels(store: &Store, modeler: &Option<Object>, list: &json::List)
{
	fn add_label(store: &Store, modeler: &Option<Object>, object: &Json)
	{
		let mut entries = ~[];
		if modeler.is_some()
		{
			entries.push((~"gnos:modeler-subject", modeler.get()));
		}
		entries.push((~"gnos:target",		IriValue(get_str(object, ~"target-id"))));
		entries.push((~"gnos:label",		StringValue(get_str(object, ~"label"), ~"")));
		entries.push((~"gnos:level", 		IntValue(get_i64(object, ~"level"))));
		entries.push((~"gnos:sort_key",	StringValue(get_str(object, ~"sort-key"), ~"")));
		do optional_str(object, ~"style") |value| 		{entries.push((~"gnos:style", StringValue(value, ~"")))};
		do optional_str(object, ~"predicate") |value|	{entries.push((~"gnos:predicate", StringValue(value, ~"")))};
		
		store.add(get_blank_name(store, ~"label"), entries);
	}
	
	for list.each |label|
	{
		add_label(store, modeler, label);
	}
}

priv fn add_gauges(store: &Store, modeler: &Option<Object>, list: &json::List)
{
	fn add_gauge(store: &Store, modeler: &Option<Object>, object: &Json)
	{
		let mut entries = ~[];
		if modeler.is_some()
		{
			entries.push((~"gnos:modeler-subject", modeler.get()));
		}
		entries.push((~"gnos:target",		IriValue(get_str(object, ~"entity-id"))));
		entries.push((~"gnos:gauge", 		FloatValue(get_f64(object, ~"value"))));
		entries.push((~"gnos:title",		StringValue(get_str(object, ~"label"), ~"")));
		entries.push((~"gnos:level", 		IntValue(get_i64(object, ~"level"))));
		entries.push((~"gnos:sort_key",	StringValue(get_str(object, ~"sort-key"), ~"")));
		do optional_str(object, ~"style") |value| 		{entries.push((~"gnos:style", StringValue(value, ~"")))};
		do optional_str(object, ~"predicate") |value|	{entries.push((~"gnos:predicate", StringValue(value, ~"")))};
		
		store.add(get_blank_name(store, ~"gauge"), entries);
	}
	
	for list.each |gauge|
	{
		add_gauge(store, modeler, gauge);
	}
}

priv fn add_details(store: &Store, modeler: &Option<Object>, list: &json::List)
{
	fn add_detail(store: &Store, modeler: &Option<Object>, object: &Json)
	{
		let mut entries = ~[];
		if modeler.is_some()
		{
			entries.push((~"gnos:modeler-subject", modeler.get()));
		}
		entries.push((~"gnos:target",		IriValue(get_str(object, ~"entity-id"))));
		entries.push((~"gnos:title",		StringValue(get_str(object, ~"label"), ~"")));
		entries.push((~"gnos:detail",		StringValue(get_str(object, ~"detail"), ~"")));
		entries.push((~"gnos:open",		StringValue(get_str(object, ~"open"), ~"")));
		entries.push((~"gnos:sort_key",	StringValue(get_str(object, ~"sort-key"), ~"")));
		entries.push((~"gnos:key",		StringValue(get_str(object, ~"id"), ~"")));
		
		store.add(get_blank_name(store, ~"detail"), entries);
	}
	
	for list.each |detail|
	{
		add_detail(store, modeler, detail);
	}
}

priv fn add_relations(store: &Store, modeler: &Option<Object>, list: &json::List)
{
	fn add_label(store: &Store, modeler: &Option<Object>, object: &Json, entries: &mut ~[(~str, Object)], target: &str, position: ~str)
	{
		do optional_object(object, position + ~"-label") |sub_object|
		{
			let mut sub_entries = ~[];
			if modeler.is_some()
			{
				sub_entries.push((~"gnos:modeler-subject", modeler.get()));
			}
			sub_entries.push((~"gnos:target",		BlankValue(target.to_unique())));
			sub_entries.push((~"gnos:label",		StringValue(get_str(sub_object, ~"label"), ~"")));
			sub_entries.push((~"gnos:level", 		IntValue(get_i64(sub_object, ~"level"))));
			sub_entries.push((~"gnos:sort_key",	StringValue(~"a", ~"")));
			do optional_str(sub_object, ~"style") |value| {sub_entries.push((~"gnos:style", StringValue(value, ~"")))};
			
			let sub_target = get_blank_name(store, ~"label");
			store.add(sub_target, sub_entries);
			entries.push((fmt!("gnos:%s_info", position), BlankValue(sub_target)));
		}
	}
	
	fn add_relation(store: &Store, modeler: &Option<Object>, object: &Json)
	{
		let target = get_blank_name(store, ~"relation");
		
		let mut entries = ~[];
		if modeler.is_some()
		{
			entries.push((~"gnos:modeler-subject", modeler.get()));
		}
		entries.push((~"gnos:left",		IriValue(get_str(object, ~"left-entity-id"))));
		entries.push((~"gnos:right",	IriValue(get_str(object, ~"right-entity-id"))));
		do optional_str(object, ~"style") |value| {entries.push((~"gnos:style", StringValue(value, ~"")))};
		
		add_label(store, modeler, object, &mut entries, target, ~"left");
		add_label(store, modeler, object, &mut entries, target, ~"middle");
		add_label(store, modeler, object, &mut entries, target, ~"right");
		
		store.add(target, entries);
	}
	
	for list.each |relation|
	{
		add_relation(store, modeler, relation);
	}
}

priv fn add_alerts(store: &Store, list: &json::List)
{
	fn open_alert(store: &Store, object: &Json)
	{
		let alert = model::Alert
		{
			target: get_str(object, ~"entity-id"),
			id: get_str(object, ~"key"),
			mesg: get_str(object, ~"mesg"),
			resolution: get_str(object, ~"resolution"),
			level: get_str(object, ~"kind"),
		};
		model::open_alert(store, &alert);
	}
	
	fn close_alert(store: &Store, object: &Json)
	{
		model::close_alert(store, get_str(object, ~"entity-id"), get_str(object, ~"key"));
	}
	
	for list.each |alert|
	{
		if has_value(alert, ~"mesg")
		{
			open_alert(store, alert);
		}
		else
		{
			close_alert(store, alert);
		}
	}
}

priv fn add_samples(options: &Options, samples_chan: SamplesChan, list: &json::List)
{
	let path = get_sparkline_script(options);
	let context = mustache::Context(~".", ~"");
	let template = context.compile_file(path.to_str());
	
	let mut script = ~"";
	for list.each |sample|
	{
		let name = get_str(sample, ~"name");
		samples_chan.send(samples::AddSample(~"snmp", copy name, get_float(sample, ~"value"), samples_capacity));
		script += build_sparkline(options, samples_chan, name, get_str(sample, ~"units"), template);
	}
	
	if script.is_not_empty()
	{
		run_r_script(script);
	}
}

priv fn add_charts(options: &Options, samples_chan: SamplesChan, list: &json::List)
{
	let mut charts = ~[];
	
	let root = os::make_absolute(&options.root);
	let root = root.push("generated");
	for list.each |chart|
	{
		let path = root.push(fmt!("%s.png", get_str(chart, ~"name")));
		charts.push(samples::Chart 
		{
			path: path.to_str(),
			sample_sets: get_strs(chart, ~"samples"),
			legends: get_strs(chart, ~"legends"),
			interval: options.poll_rate as float,
			title: get_str(chart, ~"title"),
			y_label: get_str(chart, ~"y_label"),
		});
	}
	
	if charts.is_not_empty()
	{
		// We always create these charts. That's a bit wasteful because they don't appear on the main page.
		// However building an URL that encodes all the info neccesary to create them would be rather
		// awful. TODO: I guess samples could store a Chart struct and then use that to dynamically create
		// the charts.
		samples::create_charts(~"snmp-modeler", charts, samples_chan);
	}
}

// Creates an R script which when run will produce a sparkline chart for the named sample set.
priv fn build_sparkline(options: &Options, samples_chan: SamplesChan, name: &str, units: ~str, template: Template) -> ~str
{
	let port = Port();
	let chan = Chan(&port);
	samples_chan.send(samples::GetSampleSet(name.to_unique(), chan));
	let (buffer, _num_adds) = port.recv();
	
	if (buffer.len() > 1)
	{
		let mut path = os::make_absolute(&options.root);
		path = path.push("generated");
		path = path.push(fmt!("%s.png", name));
		
		let context = HashMap();
		context.insert(@~"samples", mustache::Str(@str::connect(do iter::map_to_vec(&buffer) |s| {s.to_str()}, ", ")));
		context.insert(@~"file", mustache::Str(@path.to_str()));
		context.insert(@~"width", mustache::Str(@~"150"));
		context.insert(@~"height", mustache::Str(@~"50"));
		context.insert(@~"label", mustache::Str(@units));
		
		template.render_data(mustache::Map(context))
	}
	else
	{
		~""
	}
}

priv fn run_r_script(script: &str)
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
					
					let program = run::start_program("Rscript", [path.to_str()]);
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

priv fn prune_modeler(store: &Store, value: &Json) -> Option<Object>
{
	let mut mine = option::None;
	
	match *value
	{
		json::String(ref modeler) =>
		{
			mine = option::Some(StringValue(copy *modeler, ~""));
			do utils::remove_entry_if(store.subjects) |_key, value|
			{
				let entry = value.get_elt(0);
				entry.predicate == ~"http://www.gnos.org/2012/schema#modeler-subject" && entry.object == mine.get()
			}
		}
		_ =>
		{
			error!("Expected a String but found %?", value);
		}
	}
	
	mine
}

priv fn optional_str(value: &Json, key: ~str, callback: fn (value: ~str))
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::String(copy s) =>
					{
						callback(s);
					}
					_ =>
					{
						error!("Expected a String but found %?", *entry);
					}
				}
			}
		}
		_ =>
		{
			error!("Expected an Object but found %?", *value);
		}
	}
}

priv fn get_str(value: &Json, key: ~str) -> ~str
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::String(copy s) =>
					{
						s
					}
					_ =>
					{
						fail fmt!("Expected a String but found %?", *entry)
					}
				}
			}
			else
			{
				fail fmt!("%s key is missing from %?", key, value)
			}
		}
		_ =>
		{
			fail fmt!("Expected an Object but found %?", *value)
		}
	}
}

priv fn get_strs(value: &Json, key: ~str) -> ~[~str]
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::List(ref s) =>
					{
						do s.map |x|
						{
							match *x
							{
								json::String(copy s) =>
								{
									s
								}
								_ =>
								{
									fail fmt!("Expected a String but found %?", *x)
								}
							}
						}
					}
					_ =>
					{
						fail fmt!("Expected a List but found %?", *entry)
					}
				}
			}
			else
			{
				fail fmt!("%s key is missing from %?", key, value)
			}
		}
		_ =>
		{
			fail fmt!("Expected an Object but found %?", *value)
		}
	}
}

priv fn has_value(value: &Json, key: ~str) -> bool
{
	match *value
	{
		json::Object(ref object) =>
		{
			object.contains_key(&key)
		}
		_ =>
		{
			false
		}
	}
}

priv fn get_i64(value: &Json, key: ~str) -> i64
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::Number(n) =>
					{
						n as i64
					}
					_ =>
					{
						fail fmt!("Expected a Number but found %?", *entry)
					}
				}
			}
			else
			{
				fail fmt!("%s key is missing from %?", key, value)
			}
		}
		_ =>
		{
			fail fmt!("Expected an Object but found %?", *value)
		}
	}
}

priv fn get_f64(value: &Json, key: ~str) -> f64
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::Number(n) =>
					{
						n as f64
					}
					_ =>
					{
						fail fmt!("Expected a Number but found %?", *entry)
					}
				}
			}
			else
			{
				fail fmt!("%s key is missing from %?", key, value)
			}
		}
		_ =>
		{
			fail fmt!("Expected an Object but found %?", *value)
		}
	}
}

priv fn get_float(value: &Json, key: ~str) -> float
{
	get_f64(value, key) as float
}

priv fn optional_object(value: &Json, key: ~str, callback: fn (value: &Json))
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::Object(_) =>
					{
						callback(entry);
					}
					_ =>
					{
						error!("Expected a Object but found %?", *entry);
					}
				}
			}
		}
		_ =>
		{
			error!("Expected an Object but found %?", *value);
		}
	}
}

priv fn optional_list(value: &Json, key: ~str, callback: fn (value: &json::List))
{
	match *value
	{
		json::Object(ref object) =>
		{
			if object.contains_key(&key)
			{
				let entry = object.get_ref(&key);
				match *entry
				{
					json::List(ref list) =>
					{
						callback(list);
					}
					_ =>
					{
						error!("Expected a List but found %?", *entry);
					}
				}
			}
		}
		_ =>
		{
			error!("Expected an Object but found %?", *value);
		}
	}
}

priv fn get_sparkline_script(options: &Options) -> Path
{
	let path = options.root.pop();				// gnos
	let path = path.push(~"scripts");
	let path = path.push(~"sparkline.R");		// gnos/scripts/sparkline.R
	path
}
