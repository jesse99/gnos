//! Hard-coded store used when --db is on the command line.
//! 
//! Used to test the client-side code.
use model::*;
use rrdf::rrdf::*;

// TODO: In the future this should be replaced with a turtle file
// and --db should take a path to it (and maybe others).
fn setup(state_chan: comm::Chan<model::Msg>, poll_rate: u16) 
{
	comm::send(state_chan, model::UpdateMsg(~"primary", |store, _data| {add_got(store, state_chan, poll_rate); true}, ~""));
//	add_alerts(state_chan);
}

//priv fn update_got(state_chan: comm::Chan<model::Msg>, winterfell_loyalty: ~str, wl: f64, kings_landing_loyalty: ~str, kl: f64, poll_rate: u16)
//{
//	libc::funcs::posix88::unistd::sleep(poll_rate as core::libc::types::os::arch::c95::c_uint);
//	
//	let delta = if wl > 0.4f64 && kl > 0.4f64 {-0.01f64} else {0.01f64};
//	let wl = wl + delta;
//	let kl = kl + delta;
//	
//	// TODO:
//	// get the loyalties updating
//	// be sure to use the info subject
//	// change level and style at certain thresholds
//	comm::send(state_chan, model::UpdateMsg(~"primary",
//		|store, _data, copy winterfell_loyalty, copy kings_landing_loyalty|
//		{
//			store.replace_triple(~[], {subject: winterfell_loyalty, predicate: ~"gnos:level", object: FloatValue(wl)});
//			store.replace_triple(~[], {subject: kings_landing_loyalty, predicate: ~"gnos:level", object: FloatValue(kl)});
//			true
//		}, ~""));
//		
//	update_got(state_chan, winterfell_loyalty, wl, kings_landing_loyalty, kl, poll_rate);
//}

priv fn add_got(store: &Store, _state_chan: comm::Chan<model::Msg>, poll_rate: u16)
{
	add_globals(store, poll_rate);
	add_entities(store);
	add_infos(store);
	
	// relations
//	add_relation(store, wall, winterfell, ~"link", ~"road");
//	add_relation(store, kings_landing, winterfell, ~"route", ~"king's road");
	
	// details
//	let map_summary = get_blank_name(store, ~"summary");
//	store.add(map_summary, ~[
//		(~"gnos:title",       StringValue(~"Game of Thrones", ~"")),
//		(~"gnos:target",    IriValue(~"gnos:map")),
//		(~"gnos:detail",    StringValue(~"<p class='summary'>A song of ice and fire.</p>", ~"")),
//		(~"gnos:weight",  FloatValue(0.9f64)),
//		(~"gnos:open",     StringValue(~"always", ~"")),
//		(~"gnos:key",     StringValue(~"d1", ~"")),
//	]);
//	
//	let wall_summary = get_blank_name(store, ~"summary");
//	store.add(wall_summary, ~[
//		(~"gnos:title",       StringValue(~"Night's Watch Vows", ~"")),
//		(~"gnos:target",    IriValue(wall)),
//		(~"gnos:detail",    StringValue(~"<p class='summary'><em>Night gathers, and now my watch begins. It shall not end until my death. I shall take no wife, hold no lands, father no children. I shall wear no crowns and win no glory. I shall live and die at my post. I am the sword in the darkness. I am the watcher on the walls. I am the fire that burns against the cold, the light that brings the dawn, the horn that wakes the sleepers, the shield that guards the realms of men. I pledge my life and honor to the Night's Watch, for this night and all nights to come.</em></p>", ~"")),
//		(~"gnos:weight",  FloatValue(0.1f64)),
//		(~"gnos:open",     StringValue(~"yes", ~"")),
//		(~"gnos:key",     StringValue(~"d2", ~"")),
//	]);
//	
//	let winterfell_summary = get_blank_name(store, ~"summary");
//	store.add(winterfell_summary, ~[
//		(~"gnos:title",       StringValue(~"Winterfell Description", ~"")),
//		(~"gnos:target",    IriValue(winterfell)),
//		(~"gnos:detail",    StringValue(~"<p class='summary'>Winterfell is the ancestral castle and seat of power of House Stark and is considered to be the capital of the North. It is located in the center of the northern province of the Seven Kingdoms, on the Kingsroad that runs from King's Landing to the Wall.</p>", ~"")),
//		(~"gnos:weight",  FloatValue(0.9f64)),
//		(~"gnos:open",     StringValue(~"yes", ~"")),
//		(~"gnos:key",     StringValue(~"d3", ~"")),
//	]);
//	
//	let kings_landing_summary = get_blank_name(store, ~"summary");
//	store.add(kings_landing_summary, ~[
//		(~"gnos:title",       StringValue(~"King's Landing Description", ~"")),
//		(~"gnos:target",    IriValue(copy kings_landing)),
//		(~"gnos:detail",    StringValue(~"<p class='summary'>King's Landing is the capital of the Seven Kingdoms, located on the east coast of Westeros, overlooking Blackwater Bay. It is the site of the Iron Throne and the Red Keep, the seat of the King. The main city is surrounded by a wall, manned by the City Watch of King's Landing, also known as the Gold Cloaks. Poorer smallfolk build shanty settlements outside the city. King's Landing is extremely populous, but rather unsightly and dirty compared to other cities. The stench of the city's waste can be smelled far beyond its walls. It is the principal harbor of the Seven Kingdoms, rivaled only by Oldtown.</p>", ~"")),
//		(~"gnos:weight",  FloatValue(0.9f64)),
//		(~"gnos:open",     StringValue(~"yes", ~"")),
//		(~"gnos:key",     StringValue(~"d4", ~"")),
//	]);
//	
//	let kings_landing_places = get_blank_name(store, ~"places");
//	store.add(kings_landing_places, ~[
//		(~"gnos:title",       StringValue(~"King's Landing Places", ~"")),
//		(~"gnos:target",    IriValue(kings_landing)),
//		(~"gnos:detail",    StringValue(~"<dl><dt>Red Keep</dt><dd>The royal castle located on top of Aegon's Hill</dd><dt>Great Sept of Baelor</dt><dd>Where the Most Devout convene with the High Septon. It is the holiest sept of the Seven. It is located on Visenya's Hill.</dd><dt>Dragonpit</dt><dd>A huge dome, now collapsed, that used to hold the Targaryen dragons. Its bronze doors have not been opened for more than a century. It is found on Rhaenys's Hill. The Street of Sisters runs between it and the Great Sept of Baelor.</dd><dt>Alchemist's Guildhall</dt><dd>Beneath Rhaenys's Hill, stretching right to the foot of Visenya's Hill, along the Street of Sisters. Beneath it is where the Alchemists create and store the wildfire.</dd><dt>Flea Bottom</dt>	<dd>Slum area of King's Landing, a downtrodden area of town. It has pot-shops along the alleys where one can get a 'bowl o' brown.' It has a stench of pigsties and stables, tanner's sheds mixed in the smell of winesinks and whorehouses.</dd></dl>", ~"")),
//		(~"gnos:weight",  FloatValue(0.4f64)),
//		(~"gnos:open",     StringValue(~"no", ~"")),
//		(~"gnos:key",     StringValue(~"d5", ~"")),
//	]);
	
	// update_got calls libc sleep so it needs its own thread
//	do task::spawn_sched(task::SingleThreaded) {update_got(state_chan, winterfell_loyalty, 0.6f64, kings_landing_loyalty, 0.9f64, poll_rate);}
}

priv fn add_globals(store: &Store, poll_rate: u16)
{
	store.add(~"map:primary/globals", ~[
		(~"gnos:poll_interval",	IntValue(poll_rate as i64)),
		(~"gnos:last_update", 		DateTimeValue(std::time::now())),
		// TODO: should add some options
	]);
}

priv fn add_entities(store: &Store)
{
	let wall = ~"map:primary/entities/wall";
	store.add(wall, ~[
		(~"gnos:entity",	StringValue(~"The Wall", ~"")),
		(~"gnos:style",		StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	let winterfell = ~"map:primary/entities/winterfell";
	store.add(winterfell, ~[
		(~"gnos:entity",	StringValue(~"Winterfell", ~"")),
		(~"gnos:style",		StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	let kings_landing = ~"map:primary/entities/kings_landing";
	store.add(kings_landing, ~[
		(~"gnos:entity",		StringValue(~"King's Landing", ~"")),
		(~"gnos:style",			StringValue(~"font-size:x-large font-weight:bolder frame-blur:5", ~"")),
	]);
}

priv fn add_infos(store: &Store)
{
	// wall labels
	store.add(get_blank_name(store, ~"wall-label"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/wall")),
		(~"gnos:label",	StringValue(~"guards the realms of men", ~"")),
		(~"gnos:level",	IntValue(2)),
	]);
	
	// winterfell labels
	store.add(get_blank_name(store, ~"winterfell-label"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/winterfell")),
		(~"gnos:label",	StringValue(~"House Stark", ~"")),
		(~"gnos:level",	IntValue(1)),
	]);
	
	store.add(get_blank_name(store, ~"winterfell-label"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/winterfell")),
		(~"gnos:label",	StringValue(~"constructed by Brandon the Builder", ~"")),
		(~"gnos:level",	IntValue(2)),
	]);
	
	// kings_landing labels
	store.add(get_blank_name(store, ~"kings_landing-label"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/kings_landing")),
		(~"gnos:label",	StringValue(~"Capitol of Westoros", ~"")),
		(~"gnos:level",	IntValue(1)),
		(~"gnos:base_style",	StringValue(~"font-size:x-large", ~"")),
	]);
	
	// wall gauges
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/wall")),
		(~"gnos:gauge",	FloatValue(1.0f64)),
		(~"gnos:title",		StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	IntValue(2)),
	]);
	
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/wall")),
		(~"gnos:gauge",	FloatValue(0.3f64)),
		(~"gnos:title",		StringValue(~"loyalty", ~"")),
		(~"gnos:level",	IntValue(2)),
		(~"gnos:style",		StringValue(~"gauge-bar-color:orange", ~"")),
	]);
	
	// winterfell gauges
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/winterfell")),
		(~"gnos:gauge",	FloatValue(0.7f64)),
		(~"gnos:title",		StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	IntValue(2)),
	]);
	
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/winterfell")),
		(~"gnos:gauge",	FloatValue(0.1f64)),
		(~"gnos:title",		StringValue(~"loyalty", ~"")),
		(~"gnos:level",	IntValue(2)),
		(~"gnos:style",		StringValue(~"gauge-bar-color:crimson", ~"")),
	]);
	
	// king's landing gauges
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/kings_landing")),
		(~"gnos:gauge",	FloatValue(0.5f64)),
		(~"gnos:title",		StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	IntValue(2)),
	]);
	
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	IriValue(~"map:primary/entities/kings_landing")),
		(~"gnos:gauge",	FloatValue(0.9f64)),
		(~"gnos:title",		StringValue(~"loyalty", ~"")),
		(~"gnos:level",	IntValue(2)),
		(~"gnos:style",		StringValue(~"gauge-bar-color:lime", ~"")),
	]);
}

//priv fn add_alerts(state_chan: comm::Chan<model::Msg>) -> bool
//{
//	// map
//	comm::send(state_chan, model::UpdateMsg(~"alerts", |store, _msg|
//	{
//		model::open_alert(store, &Alert {device: ~"gnos:map", id: ~"m1", level: model::ErrorLevel, mesg: ~"Detonation in 5s", resolution: ~"Cut the blue wire."});
//		model::open_alert(store, &Alert {device: ~"gnos:map", id: ~"m2", level: model::WarningLevel, mesg: ~"Approaching critical mass", resolution: ~"Reduce mass."});
//		
//		model::open_alert(store, &Alert {device: ~"gnos:map", id: ~"m3", level: model::ErrorLevel, mesg: ~"Electrons are leaking", resolution: ~"Call a plumber."});
//		model::close_alert(store, ~"gnos:map", ~"m3");			// closed alert 
//		
//																		// open_alert is idempotent
//		model::open_alert(store, &Alert {device: ~"gnos:map", id: ~"m1", level: model::ErrorLevel, mesg: ~"Detonation in 5s", resolution: ~"Cut the blue wire."})
//	}, ~""));
//	
//	// devices
//	comm::send(state_chan, model::UpdateMsg(~"alerts", |store, _msg|
//	{
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w1", level: model::ErrorLevel, mesg: ~"The ocean is rising.", resolution: ~"Call King Canute."});
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w2", level: model::ErrorLevel, mesg: ~"Ghosts walk the grounds.", resolution: ~"Who you going to call?"});
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w3", level: model::WarningLevel, mesg: ~"Winter is coming.", resolution: ~"Increase the stores."});
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w4", level: model::InfoLevel, mesg: ~"Bran stubbed his toe.", resolution: ~"Call the Maester."});
//		
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w5", level: model::ErrorLevel, mesg: ~"A deserter from the Wall was found.", resolution: ~"Chop his head off."});
//		model::close_alert(store, ~"map:primary/entities/winterfell", ~"w5");	// closed alert
//		
//		model::close_alert(store, ~"map:primary/entities/winterfell", ~"w2");	// re-opened alert
//		model::open_alert(store, &Alert {device: ~"map:primary/entities/winterfell", id: ~"w2", level: model::ErrorLevel, mesg: ~"More ghosts walk the grounds.", resolution: ~"Who you going to call?"})
//	}, ~""));
//	
//	true
//}

//priv fn add_relation(store: &Store, lhs: ~str, rhs: ~str, style: ~str, label: ~str)
//{
//	let lhs_relation = get_blank_name(store, ~"lhs");
//	store.add(lhs_relation, ~[
//		(~"gnos:src",                 	IriValue(copy lhs)),
//		(~"gnos:dst",                 	IriValue(copy rhs)),
//		(~"gnos:type",                 StringValue(~"unidirectional", ~"")),
//		(~"gnos:style",                 StringValue(copy style, ~"")),
//		(~"gnos:primary_label",    StringValue(copy label, ~"")),
//		(~"gnos:secondary_label", StringValue(~"details", ~"")),
//		(~"gnos:tertiary_label",     StringValue(~"more details", ~"")),
//	]);
//	
//	let rhs_relation = get_blank_name(store, ~"rhs");
//	store.add(rhs_relation, ~[
//		(~"gnos:src",                 	IriValue(copy rhs)),
//		(~"gnos:dst",                 	IriValue(copy lhs)),
//		(~"gnos:type",                 StringValue(~"unidirectional", ~"")),
//		(~"gnos:style",                 StringValue(copy style, ~"")),
//		(~"gnos:primary_label",    StringValue(copy label, ~"")),
//		(~"gnos:secondary_label", StringValue(~"details", ~"")),
//	]);
//}
