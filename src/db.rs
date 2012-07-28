//! Hard-coded store used when --db is on the command line.
//! 
//! Used to test the client-side code.
import model;
import rrdf::*;
import rrdf::store::{store_trait};

export setup;

fn setup(state_chan: comm::chan<model::msg>) 
{
	comm::send(state_chan, model::update_msg(~"model", add_got, ~""));
}

fn add_got(store: store, _data: ~str) -> bool
{
	let wall = get_blank_name(store, ~"obj");
	let winterfell = get_blank_name(store, ~"obj");
	let knights_landing = get_blank_name(store, ~"obj");
	
	//let bran = get_blank_name(store, ~"obj");
	//let sansa = get_blank_name(store, ~"obj");
	//let sandor = get_blank_name(store, ~"obj");
	//let slayer = get_blank_name(store, ~"obj");
	//let cersei = get_blank_name(store, ~"obj");
	//let jaime = get_blank_name(store, ~"obj");
	
	// map
	store.add(~"gnos:map", ~[
		(~"gnos:object",          blank_value(wall)),
		(~"gnos:object",          blank_value(winterfell)),
		(~"gnos:object",          blank_value(knights_landing)),
		
		//(~"gnos:object",          blank_value(bran)),
		//(~"gnos:object",          blank_value(sansa)),
		//(~"gnos:object",          blank_value(sandor)),
		//(~"gnos:object",          blank_value(slayer)),
		//(~"gnos:object",          blank_value(cersei)),
		//(~"gnos:object",          blank_value(jaime)),
		
		(~"gnos:poll_interval", int_value(10)),
		(~"gnos:last_update",  dateTime_value(std::time::now())),
	]);
	
	// objects
	store.add(wall, ~[
		(~"gnos:center_x",           float_value(0.5f64)),
		(~"gnos:center_y",           float_value(0.1f64)),
		(~"gnos:primary_label",    string_value(~"The Wall", ~"")),
		(~"gnos:tertiary_label",     string_value(~"guards the realms of men", ~"")),
	]);
	
	store.add(winterfell, ~[
		(~"gnos:center_x",           float_value(0.4f64)),
		(~"gnos:center_y",           float_value(0.3f64)),
		(~"gnos:style",                 string_value(~"large", ~"")),
		(~"gnos:primary_label",    string_value(~"Winterfell", ~"")),
		(~"gnos:secondary_label", string_value(~"House Stark", ~"")),
		(~"gnos:tertiary_label",     string_value(~"constructed by Brandon the Builder", ~"")),
	]);
	
	store.add(knights_landing, ~[
		(~"gnos:center_x",           float_value(0.6f64)),
		(~"gnos:center_y",           float_value(0.7f64)),
		(~"gnos:style",                 string_value(~"xlarge", ~"")),
		(~"gnos:primary_label",    string_value(~"Knight's Landing", ~"")),
		(~"gnos:secondary_label", string_value(~"Capitol of Westoros", ~"")),
	]);
	
	add_relation(store, ~"gnos:undirected", wall, winterfell, ~"thick", ~"road");
	add_relation(store, ~"gnos:undirected", knights_landing, winterfell, ~"xxthick", ~"king's road");
	true
}

fn add_relation(store: store, predicate: ~str, lhs: ~str, rhs: ~str, style: ~str, label: ~str)
{
	let lhs_relation = get_blank_name(store, ~"lhs");
	store.add_triple(~[], {subject: lhs, predicate: predicate, object: blank_value(lhs_relation)});
	store.add(lhs_relation, ~[
		(~"gnos:peer",                 blank_value(rhs)),
		(~"gnos:style",                 string_value(style, ~"")),
		(~"gnos:primary_label",    string_value(label, ~"")),
		(~"gnos:secondary_label", string_value(~"details", ~"")),
		(~"gnos:tertiary_label",     string_value(~"more details", ~"")),
	]);
	
	let rhs_relation = get_blank_name(store, ~"rhs");
	store.add_triple(~[], {subject: rhs, predicate: predicate, object: blank_value(rhs_relation)});
	store.add(rhs_relation, ~[
		(~"gnos:peer",                 blank_value(lhs)),
		(~"gnos:style",                 string_value(style, ~"")),
		(~"gnos:primary_label",    string_value(label, ~"")),
		(~"gnos:secondary_label", string_value(~"details", ~"")),
	]);
}
