//! Hard-coded store used when --db is on the command line.
//! 
//! Used to test the client-side code.
import model;
import rrdf::*;
import rrdf::store::{store_trait};

export setup;

fn setup(state_chan: comm::chan<model::msg>) 
{
	comm::send(state_chan, model::update_msg(~"primary", add_got, ~""));
}

fn add_got(store: store, _data: ~str) -> bool
{
	// map
	store.add(~"gnos:map", ~[
		(~"gnos:poll_interval", int_value(10)),
		(~"gnos:last_update",  dateTime_value(std::time::now())),
	]);
	
	// devices
	let wall = ~"devices:wall";
	store.add(wall, ~[
		(~"gnos:center_x",           float_value(0.5f64)),
		(~"gnos:center_y",           float_value(0.12f64)),
		(~"gnos:style",                 string_value(~"host", ~"")),
		(~"gnos:primary_label",    string_value(~"The Wall", ~"")),
		(~"gnos:tertiary_label",     string_value(~"guards the realms of men", ~"")),
	]);
	
	let winterfell = ~"devices:winterfell";
	store.add(winterfell, ~[
		(~"gnos:center_x",           float_value(0.2f64)),
		(~"gnos:center_y",           float_value(0.3f64)),
		(~"gnos:style",                 string_value(~"router", ~"")),
		(~"gnos:primary_label",    string_value(~"Winterfell", ~"")),
		(~"gnos:secondary_label", string_value(~"House Stark", ~"")),
		(~"gnos:tertiary_label",     string_value(~"constructed by Brandon the Builder", ~"")),
	]);
	
	let knights_landing = ~"devices:knights_landing";
	store.add(knights_landing, ~[
		(~"gnos:center_x",           float_value(0.6f64)),
		(~"gnos:center_y",           float_value(0.77f64)),
		(~"gnos:style",                 string_value(~"switch", ~"")),
		(~"gnos:primary_label",    string_value(~"Knight's Landing", ~"")),
		(~"gnos:secondary_label", string_value(~"Capitol of Westoros", ~"")),
	]);
	
	// relations
	add_relation(store, wall, winterfell, ~"link", ~"road");
	add_relation(store, knights_landing, winterfell, ~"route", ~"king's road");
	
	// indicators
	let wall_mf = get_blank_name(store, ~"meter");
	store.add(wall_mf, ~[
		(~"gnos:meter",        string_value(~"MF", ~"")),
		(~"gnos:target",        iri_value(wall)),
		(~"gnos:level",          float_value(1.0f64)),
		(~"gnos:description", string_value(~"male/female ratio", ~"")),
	]);
	
	let wall_loyalty = get_blank_name(store, ~"meter");
	store.add(wall_loyalty, ~[
		(~"gnos:meter",        string_value(~"loyalty", ~"")),
		(~"gnos:target",        iri_value(wall)),
		(~"gnos:level",          float_value(0.0f64)),
		(~"gnos:description", string_value(~"loyalty to the crown", ~"")),
	]);
	
	let winterfell_mf = get_blank_name(store, ~"meter");
	store.add(winterfell_mf, ~[
		(~"gnos:meter",        string_value(~"MF", ~"")),
		(~"gnos:target",        iri_value(winterfell)),
		(~"gnos:level",          float_value(0.7f64)),
		(~"gnos:description", string_value(~"male/female ratio", ~"")),
	]);
	
	let winterfell_loyalty = get_blank_name(store, ~"meter");
	store.add(winterfell_loyalty, ~[
		(~"gnos:meter",        string_value(~"loyalty", ~"")),
		(~"gnos:target",        iri_value(winterfell)),
		(~"gnos:level",          float_value(0.6f64)),
		(~"gnos:description", string_value(~"loyalty to the crown", ~"")),
	]);
	
	let knights_landing_mf = get_blank_name(store, ~"meter");
	store.add(knights_landing_mf, ~[
		(~"gnos:meter",        string_value(~"MF", ~"")),
		(~"gnos:target",        iri_value(knights_landing)),
		(~"gnos:level",          float_value(0.5f64)),
		(~"gnos:description", string_value(~"male/female ratio", ~"")),
	]);
	
	let knights_landing_loyalty = get_blank_name(store, ~"meter");
	store.add(knights_landing_loyalty, ~[
		(~"gnos:meter",        string_value(~"loyalty", ~"")),
		(~"gnos:target",        iri_value(knights_landing)),
		(~"gnos:level",          float_value(0.9f64)),
		(~"gnos:description", string_value(~"loyalty to the crown", ~"")),
	]);
	
	true
}

fn add_relation(store: store, lhs: ~str, rhs: ~str, style: ~str, label: ~str)
{
	let lhs_relation = get_blank_name(store, ~"lhs");
	store.add(lhs_relation, ~[
		(~"gnos:src",                 	iri_value(lhs)),
		(~"gnos:dst",                 	iri_value(rhs)),
		(~"gnos:type",                 string_value(~"unidirectional", ~"")),
		(~"gnos:style",                 string_value(style, ~"")),
		(~"gnos:primary_label",    string_value(label, ~"")),
		(~"gnos:secondary_label", string_value(~"details", ~"")),
		(~"gnos:tertiary_label",     string_value(~"more details", ~"")),
	]);
	
	let rhs_relation = get_blank_name(store, ~"rhs");
	store.add(rhs_relation, ~[
		(~"gnos:src",                 	iri_value(rhs)),
		(~"gnos:dst",                 	iri_value(lhs)),
		(~"gnos:type",                 string_value(~"unidirectional", ~"")),
		(~"gnos:style",                 string_value(style, ~"")),
		(~"gnos:primary_label",    string_value(label, ~"")),
		(~"gnos:secondary_label", string_value(~"details", ~"")),
	]);
}
