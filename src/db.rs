//! Hard-coded store used when --db is on the command line.
//! 
//! Used to test the client-side code.
use model::*;
use rrdf::rrdf::*;

// TODO: In the future this should be replaced with a turtle file
// and --db should take a path to it (and maybe others).
pub fn setup(state_chan: comm::Chan<model::Msg>, poll_rate: u16) 
{
	comm::send(state_chan, model::UpdateMsg(~"primary", |store, _data| {add_got(store, state_chan, poll_rate); true}, ~""));
	add_alerts(state_chan);
}

// This is designed to test live updating of views. What we do is degrade the loyalty (to the crown) of
// winterfell and knight's landing. The color and level settings of the layalty gauges are adjusted
// based on the current loyalty value.
priv fn update_got(state_chan: comm::Chan<model::Msg>, winterfell_loyalty_subject: ~str, winterfell_loyalty_value: f64, kings_landing_loyalty_subject: ~str, kings_landing_loyalty_value: f64, poll_rate: u16)
{
	fn degrade_loyalty(value: f64, delta: f64) -> f64
	{
		if value >= delta
		{
			value - delta
		}
		else
		{
			1.0f64
		}
	}
	
	fn gauge_state(value: f64) -> (~str, i64)
	{
		if value >= 0.8f64
		{
			(~"gauge-bar-color:lime", 2)
		}
		else if value >= 0.7f64
		{
			(~"gauge-bar-color:deepskyblue", 2)
		}
		else if value >= 0.5f64
		{
			(~"gauge-bar-color:lightsalmon", 1)
		}
		else
		{
			(~"gauge-bar-color:red", 0)
		}
	}
	
	libc::funcs::posix88::unistd::sleep(poll_rate as core::libc::types::os::arch::c95::c_uint);
	
	let winterfell_loyalty_value = degrade_loyalty(winterfell_loyalty_value, 0.2f64);
	let kings_landing_loyalty_value = degrade_loyalty(kings_landing_loyalty_value, 0.1f64);
	error!("winterfell_loyalty_value = %?", winterfell_loyalty_value);
	error!("kings_landing_loyalty_value = %?", kings_landing_loyalty_value);
	
	comm::send(state_chan, model::UpdateMsg(~"primary",
		|store, _data, copy winterfell_loyalty_subject, copy kings_landing_loyalty_subject|
		{
			store.replace_triple(~[], {subject: copy winterfell_loyalty_subject, predicate: ~"gnos:gauge", object: @FloatValue(winterfell_loyalty_value)});
			store.replace_triple(~[], {subject: copy kings_landing_loyalty_subject, predicate: ~"gnos:gauge", object: @FloatValue(kings_landing_loyalty_value)});
			
			let (style, level) = gauge_state(winterfell_loyalty_value);
			store.replace_triple(~[], {subject: copy winterfell_loyalty_subject, predicate: ~"gnos:level", object: @IntValue(level)});
			store.replace_triple(~[], {subject: copy winterfell_loyalty_subject, predicate: ~"gnos:style", object: @StringValue(style, ~"")});
			
			let (style, level) = gauge_state(kings_landing_loyalty_value);
			store.replace_triple(~[], {subject: copy kings_landing_loyalty_subject, predicate: ~"gnos:level", object: @IntValue(level)});
			store.replace_triple(~[], {subject: copy kings_landing_loyalty_subject, predicate: ~"gnos:style", object: @StringValue(style, ~"")});
			true
		}, ~""));
		
	update_got(state_chan, winterfell_loyalty_subject, winterfell_loyalty_value, kings_landing_loyalty_subject, kings_landing_loyalty_value, poll_rate);
}

priv fn add_got(store: &Store, state_chan: comm::Chan<model::Msg>, poll_rate: u16)
{
	add_globals(store, poll_rate);
	add_entities(store);
	add_infos(store, state_chan, poll_rate);
	add_relations(store);
	add_details(store);
}

priv fn add_details(store: &Store)
{
	// markdown
	store.add(get_blank_name(store, ~"wall-detail"), ~[
		(~"gnos:target",		@IriValue(~"entities:wall")),
		(~"gnos:title",			@StringValue(~"Vows", ~"")),
		(~"gnos:detail",		@StringValue(~"*Night gathers, and now my watch begins. It shall not end until my death. I shall take no wife, hold no lands, father no children. I shall wear no crowns and win no glory. I shall live and die at my post. I am the sword in the darkness. I am the watcher on the walls. I am the fire that burns against the cold, the light that brings the dawn, the horn that wakes the sleepers, the shield that guards the realms of men. I pledge my life and honor to the Night's Watch, for this night and all nights to come.*", ~"")),
		(~"gnos:open",		@StringValue(~"always", ~"")),
		(~"gnos:sort_key",	@StringValue(~"1", ~"")),
		(~"gnos:key",			@StringValue(~"w1", ~"")),
	]);
	
	// accordion
	store.add(get_blank_name(store, ~"kings_landing-detail"), ~[
		(~"gnos:target",		@IriValue(~"entities:kings_landing")),
		(~"gnos:title",			@StringValue(~"King's Landing Description", ~"")),
		(~"gnos:detail",		@StringValue(~"King's Landing is the capital of the Seven Kingdoms, located on the east coast of Westeros, overlooking Blackwater Bay. It is the site of the Iron Throne and the Red Keep, the seat of the King. The main city is surrounded by a wall, manned by the City Watch of King's Landing, also known as the Gold Cloaks. Poorer smallfolk build shanty settlements outside the city. King's Landing is extremely populous, but rather unsightly and dirty compared to other cities. The stench of the city's waste can be smelled far beyond its walls. It is the principal harbor of the Seven Kingdoms, rivaled only by Oldtown.", ~"")),
		(~"gnos:open",		@StringValue(~"yes", ~"")),
		(~"gnos:sort_key",	@StringValue(~"1", ~"")),
		(~"gnos:key",			@StringValue(~"a1", ~"")),
	]);
	
	store.add(get_blank_name(store, ~"kings_landing-detail"), ~[
		(~"gnos:target",		@IriValue(~"entities:kings_landing")),
		(~"gnos:title",			@StringValue(~"King's Landing Places", ~"")),
		(~"gnos:detail",		@StringValue(~"- **Red Keep** The royal castle located on top of Aegon's Hill.\\n- **Great Sept of Baelor** Where the Most Devout convene with the High Septon. It is the holiest sept of the Seven. It is located on Visenya's Hill.\\n- **Dragonpit** A huge dome, now collapsed, that used to hold the Targaryen dragons. Its bronze doors have not been opened for more than a century. It is found on Rhaenys's Hill. The Street of Sisters runs between it and the Great Sept of Baelor.\\n- **Alchemist's Guildhall** Beneath Rhaenys's Hill, stretching right to the foot of Visenya's Hill, along the Street of Sisters. Beneath it is where the Alchemists create and store the wildfire.\\n- **Flea Bottom** Slum area of King's Landing, a downtrodden area of town. It has pot-shops along the alleys where one can get a `bowl o' brown.` It has a stench of pigsties and stables, tanner's sheds mixed in the smell of winesinks and whorehouses.", ~"")),
		(~"gnos:open",		@StringValue(~"no", ~"")),
		(~"gnos:sort_key",	@StringValue(~"2", ~"")),
		(~"gnos:key",			@StringValue(~"a2", ~"")),
	]);
	
	// markdown
	store.add(get_blank_name(store, ~"winterfell-detail"), ~[
		(~"gnos:target",	@IriValue(~"entities:winterfell")),
		(~"gnos:title",		@StringValue(~"People", ~"")),
		(~"gnos:detail",	@StringValue(~"{
	\"style\": \"plain\",
	\"header\": [\"Name\", \"Nickname\", \"Wolf\"],
	\"rows\": [
		[\"Jon\", \"Lord\", \"Ghost\"],
		[\"Arya\", \"Underfoot\", \"Nymeria\"],
		[\"Sansa\", \"\", \"Lady\"]
	]
}", ~"")),
		(~"gnos:open",	@StringValue(~"always", ~"")),
		(~"gnos:sort_key",@StringValue(~"1", ~"")),
		(~"gnos:key",		@StringValue(~"w1", ~"")),
	]);
}

priv fn add_relations(store: &Store)
{
	add_relation(store, ~"entities:wall", ~"entities:winterfell", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:winterfell", ~"entities:hornwood", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:hornwood", ~"entities:dreadfort", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:dreadfort", ~"entities:karhold", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:winterfell", ~"entities:white_harbor", ~"line-type:normal", ~"", ~"");
	
	add_relation(store, ~"entities:white_harbor", ~"entities:moat_cailin", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:winterfell", ~"entities:moat_cailin", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:moat_cailin", ~"entities:harenhal", ~"line-width:3 line-color:purple line-type:normal", ~"king's road", ~"straight and true");
	add_relation(store, ~"entities:harenhal", ~"entities:kings_landing", ~"line-width:3 line-color:purple line-type:normal", ~"king's road", ~"straight and true");
	
	add_relation(store, ~"entities:kings_landing", ~"entities:lannisport", ~"line-width:3 line-type:normal", ~"gold road", ~"");
	add_relation(store, ~"entities:lannisport", ~"entities:crakehall", ~"line-type:normal", ~"", ~"");
	add_relation(store, ~"entities:lannisport", ~"entities:clegane_hall", ~"line-type:normal", ~"", ~"");
	
	add_relation(store, ~"entities:kings_landing", ~"entities:highgarden", ~"line-width:3 line-type:normal", ~"rose road", ~"");
	add_relation(store, ~"entities:highgarden", ~"entities:oldtown", ~"line-type:normal", ~"", ~"");
}

priv fn add_globals(store: &Store, poll_rate: u16)
{
	store.add(~"map:primary/globals", ~[
		(~"gnos:poll_interval",	@IntValue(poll_rate as i64)),
		(~"gnos:last_update", 		@DateTimeValue(std::time::now())),
		// TODO: should add some options
	]);
}

priv fn add_entities(store: &Store)
{
	store.add(~"entities:wall", ~[
		(~"gnos:entity",	@StringValue(~"The Wall", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:winterfell", ~[
		(~"gnos:entity",	@StringValue(~"Winterfell", ~"")),
		(~"gnos:style",		@StringValue(~"font-size:large font-weight:bolder frame-blur:5 node-mass:3", ~"")),
	]);
	
	store.add(~"entities:dreadfort", ~[
		(~"gnos:entity",	@StringValue(~"The Dreadfort", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:karhold", ~[
		(~"gnos:entity",	@StringValue(~"Karhold", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:hornwood", ~[
		(~"gnos:entity",	@StringValue(~"Hornwood", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:white_harbor", ~[
		(~"gnos:entity",	@StringValue(~"White Harbor", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:moat_cailin", ~[
		(~"gnos:entity",	@StringValue(~"Moat Cailin", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:harenhal", ~[
		(~"gnos:entity",	@StringValue(~"Harenhal", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:kings_landing", ~[
		(~"gnos:entity",		@StringValue(~"King's Landing", ~"")),
		(~"gnos:style",			@StringValue(~"font-size:x-large font-weight:bolder frame-blur:5 node-mass:5", ~"")),
	]);
	
	store.add(~"entities:lannisport", ~[
		(~"gnos:entity",	@StringValue(~"Lannisport", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:crakehall", ~[
		(~"gnos:entity",	@StringValue(~"Crakehall", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:clegane_hall", ~[
		(~"gnos:entity",	@StringValue(~"Clegane Hall", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:highgarden", ~[
		(~"gnos:entity",	@StringValue(~"Highgarden", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
	
	store.add(~"entities:oldtown", ~[
		(~"gnos:entity",	@StringValue(~"Oldtown", ~"")),
		(~"gnos:style",		@StringValue(~"font-weight:bolder frame-blur:5", ~"")),
	]);
}

priv fn add_infos(store: &Store, state_chan: comm::Chan<model::Msg>, poll_rate: u16)
{
	// wall labels
	store.add(get_blank_name(store, ~"wall-label"), ~[
		(~"gnos:target",	@IriValue(~"entities:wall")),
		(~"gnos:label",	@StringValue(~"guards the realms of men", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"1", ~"")),
	]);
	
	// winterfell labels
	store.add(get_blank_name(store, ~"winterfell-label"), ~[
		(~"gnos:target",	@IriValue(~"entities:winterfell")),
		(~"gnos:label",	@StringValue(~"House Stark", ~"")),
		(~"gnos:level",	@IntValue(1)),
		(~"gnos:sort_key",@StringValue(~"1", ~"")),
	]);
	
	store.add(get_blank_name(store, ~"winterfell-label"), ~[
		(~"gnos:target",	@IriValue(~"entities:winterfell")),
		(~"gnos:label",	@StringValue(~"constructed by Brandon the Builder", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"2", ~"")),
	]);
	
	// kings_landing labels
	store.add(get_blank_name(store, ~"kings_landing-label"), ~[
		(~"gnos:target",	@IriValue(~"entities:kings_landing")),
		(~"gnos:label",	@StringValue(~"Capitol of Westoros", ~"")),
		(~"gnos:level",	@IntValue(1)),
		(~"gnos:sort_key",@StringValue(~"1", ~"")),
	]);
	
	// wall gauges
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	@IriValue(~"entities:wall")),
		(~"gnos:gauge",	@FloatValue(1.0f64)),
		(~"gnos:title",		@StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"3", ~"")),
	]);
	
	store.add(get_blank_name(store, ~"wall-gauge"), ~[
		(~"gnos:target",	@IriValue(~"entities:wall")),
		(~"gnos:gauge",	@FloatValue(0.3f64)),
		(~"gnos:title",		@StringValue(~"loyalty", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"4", ~"")),
		(~"gnos:style",		@StringValue(~"gauge-bar-color:orange", ~"")),
	]);
	
	// winterfell gauges
	store.add(get_blank_name(store, ~"winterfell-gauge"), ~[
		(~"gnos:target",	@IriValue(~"entities:winterfell")),
		(~"gnos:gauge",	@FloatValue(0.7f64)),
		(~"gnos:title",		@StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"3", ~"")),
	]);
	
	let winterfell_loyalty_subject = get_blank_name(store, ~"winterfell-gauge");
	let winterfell_loyalty_value = 0.8f64;
	store.add(winterfell_loyalty_subject, ~[
		(~"gnos:target",	@IriValue(~"entities:winterfell")),
		(~"gnos:gauge",	@FloatValue(winterfell_loyalty_value)),
		(~"gnos:title",		@StringValue(~"loyalty", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"4", ~"")),
		(~"gnos:style",		@StringValue(~"gauge-bar-color:lime", ~"")),
	]);
	
	// king's landing gauges
	store.add(get_blank_name(store, ~"kings_landing-gauge"), ~[
		(~"gnos:target",	@IriValue(~"entities:kings_landing")),
		(~"gnos:gauge",	@FloatValue(0.5f64)),
		(~"gnos:title",		@StringValue(~"m/f ratio", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"3", ~"")),
	]);
	
	let kings_landing_loyalty_subject = get_blank_name(store, ~"kings_landing-gauge");
	let kings_landing_loyalty_value = 0.9f64;
	store.add(kings_landing_loyalty_subject, ~[
		(~"gnos:target",	@IriValue(~"entities:kings_landing")),
		(~"gnos:gauge",	@FloatValue(kings_landing_loyalty_value)),
		(~"gnos:title",		@StringValue(~"loyalty", ~"")),
		(~"gnos:level",	@IntValue(2)),
		(~"gnos:sort_key",@StringValue(~"4", ~"")),
		(~"gnos:style",		@StringValue(~"gauge-bar-color:lime", ~"")),
	]);
	
	// update_got calls libc sleep so it needs its own thread
	do task::spawn_sched(task::SingleThreaded) |copy winterfell_loyalty_subject, copy kings_landing_loyalty_subject| {update_got(state_chan, copy winterfell_loyalty_subject, winterfell_loyalty_value, copy kings_landing_loyalty_subject, kings_landing_loyalty_value, poll_rate);}
}

priv fn add_alerts(state_chan: comm::Chan<model::Msg>) -> bool
{
	// container
	comm::send(state_chan, model::UpdateMsg(~"primary", |store, _msg|
	{
		model::open_alert(store, &Alert {target: ~"gnos:container", id: ~"m1", level: ~"error", mesg: ~"Detonation in 5s", resolution: ~"Cut the blue wire."});
		model::open_alert(store, &Alert {target: ~"gnos:container", id: ~"m2", level: ~"warning", mesg: ~"Approaching critical mass", resolution: ~"Reduce mass."});
		
		model::open_alert(store, &Alert {target: ~"gnos:container", id: ~"m3", level: ~"error", mesg: ~"Electrons are leaking", resolution: ~"Call a plumber."});
		model::close_alert(store, ~"gnos:container", ~"m3")			// closed alert 
	}, ~""));
	
	// entities
	comm::send(state_chan, model::UpdateMsg(~"primary", |store, _msg|
	{
		model::open_alert(store, &Alert {target: ~"entities:wall", id: ~"wa1", level: ~"error", mesg: ~"Night is falling.", resolution: ~"I am the fire that burns against the cold."});
		
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w1", level: ~"error", mesg: ~"The ocean is rising.", resolution: ~"Call King Canute."});
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w2", level: ~"error", mesg: ~"Ghosts walk the grounds.", resolution: ~"Who you going to call?"});
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w3", level: ~"warning", mesg: ~"Winter is coming.", resolution: ~"Increase the stores."});
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w4", level: ~"info", mesg: ~"Bran stubbed his toe.", resolution: ~"Call the Maester."});
		
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w5", level: ~"error", mesg: ~"A deserter from the Wall was found.", resolution: ~"Chop his head off."});
		model::close_alert(store, ~"entities:winterfell", ~"w5");	// closed alert
		
		model::close_alert(store, ~"entities:winterfell", ~"w2");	// re-opening alert
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w2", level: ~"error", mesg: ~"More ghosts walk the grounds.", resolution: ~"Who you going to call?"});
		
																	// open_alert is idempotent
		model::open_alert(store, &Alert {target: ~"entities:winterfell", id: ~"w1", level: ~"error", mesg: ~"Shouldn't see this.", resolution: ~"Call tech support."})
	}, ~""));
	
	true
}

priv fn add_relation(store: &Store, lhs: ~str, rhs: ~str, style: ~str, label1: ~str, label2: ~str)
{
	let relation = get_blank_name(store, ~"relation");
	
	let optional1 =
		if label1.is_not_empty()
		{
			let info1 = get_blank_name(store, ~"relation-label");
			store.add(info1, ~[
				(~"gnos:target",	@BlankValue(copy relation)),
				(~"gnos:label",	@StringValue(copy label1, ~"")),
				(~"gnos:level",	@IntValue(1)),
				(~"gnos:sort_key",@StringValue(~"1", ~"")),
			]);
			~[(~"gnos:middle_infos",	@StringValue(info1, ~""))]
		}
		else
		{
			~[]
		};
	
	let optional2 =
		if label2.is_not_empty()
		{
			let info2 = get_blank_name(store, ~"relation-label");
			store.add(info2, ~[
				(~"gnos:target",	@BlankValue(copy relation)),
				(~"gnos:label",	@StringValue(copy label2, ~"")),
				(~"gnos:level",	@IntValue(2)),
				(~"gnos:sort_key",@StringValue(~"2", ~"")),
			]);
			~[(~"gnos:left_infos",	@StringValue(info2, ~""))]
		}
		else
		{
			~[]
		};
	
	store.add(relation, ~[
		(~"gnos:left",			@IriValue(copy lhs)),
		(~"gnos:right",			@IriValue(copy rhs)),
		(~"gnos:style",			@StringValue(copy style, ~"")),
	] + optional1 + optional2);
}
