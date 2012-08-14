//! Hard-coded store used when --db is on the command line.
//! 
//! Used to test the client-side code.
import model;
import rrdf::*;
import rrdf::store::{store_trait};

export setup;

// TODO: In the future this should be replaced with a turtle file
// and --db should take a path to it (and maybe others).
fn setup(state_chan: comm::chan<model::msg>) 
{
	comm::send(state_chan, model::update_msg(~"primary", add_got, ~""));
	add_alerts(state_chan);
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
	
	let kings_landing = ~"devices:kings_landing";
	store.add(kings_landing, ~[
		(~"gnos:center_x",           float_value(0.6f64)),
		(~"gnos:center_y",           float_value(0.77f64)),
		(~"gnos:style",                 string_value(~"switch", ~"")),
		(~"gnos:primary_label",    string_value(~"King's Landing", ~"")),
		(~"gnos:secondary_label", string_value(~"Capitol of Westoros", ~"")),
	]);
	
	// relations
	add_relation(store, wall, winterfell, ~"link", ~"road");
	add_relation(store, kings_landing, winterfell, ~"route", ~"king's road");
	
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
	
	let kings_landing_mf = get_blank_name(store, ~"meter");
	store.add(kings_landing_mf, ~[
		(~"gnos:meter",        string_value(~"MF", ~"")),
		(~"gnos:target",        iri_value(kings_landing)),
		(~"gnos:level",          float_value(0.5f64)),
		(~"gnos:description", string_value(~"male/female ratio", ~"")),
	]);
	
	let kings_landing_loyalty = get_blank_name(store, ~"meter");
	store.add(kings_landing_loyalty, ~[
		(~"gnos:meter",        string_value(~"loyalty", ~"")),
		(~"gnos:target",        iri_value(kings_landing)),
		(~"gnos:level",          float_value(0.9f64)),
		(~"gnos:description", string_value(~"loyalty to the crown", ~"")),
	]);
	
	// details
	let map_summary = get_blank_name(store, ~"summary");
	store.add(map_summary, ~[
		(~"gnos:title",       string_value(~"Game of Thrones", ~"")),
		(~"gnos:target",    iri_value(~"gnos:map")),
		(~"gnos:detail",    string_value(~"<p class='summary'>A song of ice and fire.</p>", ~"")),
		(~"gnos:weight",  float_value(0.9f64)),
		(~"gnos:open",     string_value(~"always", ~"")),
	]);

	let wall_summary = get_blank_name(store, ~"summary");
	store.add(wall_summary, ~[
		(~"gnos:title",       string_value(~"Night's Watch Vows", ~"")),
		(~"gnos:target",    iri_value(wall)),
		(~"gnos:detail",    string_value(~"<p class='summary'><em>Night gathers, and now my watch begins. It shall not end until my death. I shall take no wife, hold no lands, father no children. I shall wear no crowns and win no glory. I shall live and die at my post. I am the sword in the darkness. I am the watcher on the walls. I am the fire that burns against the cold, the light that brings the dawn, the horn that wakes the sleepers, the shield that guards the realms of men. I pledge my life and honor to the Night's Watch, for this night and all nights to come.</em></p>", ~"")),
		(~"gnos:weight",  float_value(0.1f64)),
		(~"gnos:open",     string_value(~"yes", ~"")),
	]);
	
	let winterfell_summary = get_blank_name(store, ~"summary");
	store.add(winterfell_summary, ~[
		(~"gnos:title",       string_value(~"Winterfell Description", ~"")),
		(~"gnos:target",    iri_value(winterfell)),
		(~"gnos:detail",    string_value(~"<p class='summary'>Winterfell is the ancestral castle and seat of power of House Stark and is considered to be the capital of the North. It is located in the center of the northern province of the Seven Kingdoms, on the Kingsroad that runs from King's Landing to the Wall.</p>", ~"")),
		(~"gnos:weight",  float_value(0.9f64)),
		(~"gnos:open",     string_value(~"yes", ~"")),
	]);
	
	let kings_landing_summary = get_blank_name(store, ~"summary");
	store.add(kings_landing_summary, ~[
		(~"gnos:title",       string_value(~"King's Landing Description", ~"")),
		(~"gnos:target",    iri_value(kings_landing)),
		(~"gnos:detail",    string_value(~"<p class='summary'>King's Landing is the capital of the Seven Kingdoms, located on the east coast of Westeros, overlooking Blackwater Bay. It is the site of the Iron Throne and the Red Keep, the seat of the King. The main city is surrounded by a wall, manned by the City Watch of King's Landing, also known as the Gold Cloaks. Poorer smallfolk build shanty settlements outside the city. King's Landing is extremely populous, but rather unsightly and dirty compared to other cities. The stench of the city's waste can be smelled far beyond its walls. It is the principal harbor of the Seven Kingdoms, rivaled only by Oldtown.</p>", ~"")),
		(~"gnos:weight",  float_value(0.9f64)),
		(~"gnos:open",     string_value(~"yes", ~"")),
	]);
	
	let kings_landing_places = get_blank_name(store, ~"places");
	store.add(kings_landing_places, ~[
		(~"gnos:title",       string_value(~"King's Landing Places", ~"")),
		(~"gnos:target",    iri_value(kings_landing)),
		(~"gnos:detail",    string_value(~"<dl><dt>Red Keep</dt><dd>The royal castle located on top of Aegon's Hill</dd><dt>Great Sept of Baelor</dt><dd>Where the Most Devout convene with the High Septon. It is the holiest sept of the Seven. It is located on Visenya's Hill.</dd><dt>Dragonpit</dt><dd>A huge dome, now collapsed, that used to hold the Targaryen dragons. Its bronze doors have not been opened for more than a century. It is found on Rhaenys's Hill. The Street of Sisters runs between it and the Great Sept of Baelor.</dd><dt>Alchemist's Guildhall</dt><dd>Beneath Rhaenys's Hill, stretching right to the foot of Visenya's Hill, along the Street of Sisters. Beneath it is where the Alchemists create and store the wildfire.</dd><dt>Flea Bottom</dt>	<dd>Slum area of King's Landing, a downtrodden area of town. It has pot-shops along the alleys where one can get a 'bowl o' brown.' It has a stench of pigsties and stables, tanner's sheds mixed in the smell of winesinks and whorehouses.</dd></dl>", ~"")),
		(~"gnos:weight",  float_value(0.4f64)),
		(~"gnos:open",     string_value(~"no", ~"")),
	]);
	
	true
}

fn add_alerts(state_chan: comm::chan<model::msg>) -> bool
{
	// map
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg|
	{
		model::open_alert(store, {device: ~"gnos:map", id: ~"m1", level: model::error_level, mesg: ~"Detonation in 5s", resolution: ~"Cut the blue wire."});
		model::open_alert(store, {device: ~"gnos:map", id: ~"m2", level: model::warning_level, mesg: ~"Approaching critical mass", resolution: ~"Reduce mass."});
		
		model::open_alert(store, {device: ~"gnos:map", id: ~"m3", level: model::error_level, mesg: ~"Electrons are leaking", resolution: ~"Call a plumber."});
		model::close_alert(store, ~"gnos:map", ~"m3");			// closed alert 
		
																	// open_alert is idempotent
		model::open_alert(store, {device: ~"gnos:map", id: ~"m1", level: model::error_level, mesg: ~"Detonation in 5s", resolution: ~"Cut the blue wire."})
	}, ~""));
	
	// devices
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg|
	{
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w1", level: model::error_level, mesg: ~"The ocean is rising.", resolution: ~"Call King Canute."});
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w2", level: model::error_level, mesg: ~"Ghosts walk the grounds.", resolution: ~"Who you going to call?"});
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w3", level: model::warning_level, mesg: ~"Winter is coming.", resolution: ~"Increase the stores."});
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w4", level: model::info_level, mesg: ~"Bran stubbed his toe.", resolution: ~"Call the Maester."});
		
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w5", level: model::error_level, mesg: ~"A deserter from the Wall was found.", resolution: ~"Chop his head off."});
		model::close_alert(store, ~"devices:winterfell", ~"w5");	// closed alert
		
		model::close_alert(store, ~"devices:winterfell", ~"w2");	// re-opened alert
		model::open_alert(store, {device: ~"devices:winterfell", id: ~"w2", level: model::error_level, mesg: ~"More ghosts walk the grounds.", resolution: ~"Who you going to call?"})
	}, ~""));
	
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
