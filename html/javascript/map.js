"use strict";

// Maps device names ("_:device1") to objects of the form:
// {
//    center: Point
//    radius: Number
//    stroke_width: Number
//    meters: [{label: String, level: Number, description: String}]
// }
GNOS.devices = {};

GNOS.primary_data = null;
GNOS.alert_data = null;

// Thresholds for different meter levels.
GNOS.good_level		= 0.0;
GNOS.ok_level		= 0.5;
GNOS.warn_level		= 0.7;
GNOS.danger_level	= 0.8;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	var map = document.getElementById('map');
	map.addEventListener("click", handle_canvas_click);
	
	draw_initial_map();
	register_primary_query();
	register_alerts_query();
}

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (GNOS.primary_data)
	{
		redraw();
	}
	else
	{
		draw_initial_map();
	}
}

function handle_canvas_click(event)
{
	if (event.button == 0)
	{
		var pos = findPos(this);
		var pt = new Point(event.clientX - pos[0], event.clientY - pos[1]);
		
		for (var name in GNOS.devices)
		{
			var device = GNOS.devices[name];
			var disc = new Disc(device.center, device.radius);
			if (disc.intersects_pt(pt))
			{
				console.log("clicked {0}".format(name));
				break;
			}
		}
		
		event.preventDefault();
	}
}

function register_primary_query()
{
	// It's rather awkward to have all these OPTIONAL clauses, but according
	// to the spec the entire OPTIONAL block must match in order to affect 
	// the solution.
	var expr = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?center_x ?center_y ?primary_label ?secondary_label	\
	?tertiary_label ?style ?name								\
WHERE 														\
{																\
	?name gnos:center_x ?center_x .							\
	?name gnos:center_y ?center_y .							\
	OPTIONAL												\
	{															\
		?name gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:primary_label ?primary_label .			\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:secondary_label ?secondary_label .		\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:tertiary_label ?tertiary_label .				\
	}															\
}';

	var expr2 = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?src ?dst ?primary_label ?secondary_label				\
	?tertiary_label ?type ?style									\
WHERE 														\
{																\
	?rel gnos:src ?src .											\
	?rel gnos:dst ?dst .											\
	?rel gnos:type ?type .										\
	OPTIONAL												\
	{															\
		?rel gnos:style ?style .									\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:primary_label ?primary_label .				\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:secondary_label ?secondary_label .			\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:tertiary_label ?tertiary_label .				\
	}															\
}';

	var expr3 = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?label ?device ?level ?description							\
WHERE 														\
{																\
	?indicator gnos:meter ?label .								\
	?indicator gnos:target ?device .							\
	?indicator gnos:level ?level .								\
	OPTIONAL												\
	{															\
		?indicator gnos:description ?description .				\
	}															\
}';

	var expr4 = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?poll_interval ?last_update								\
WHERE 														\
{																\
	gnos:map gnos:poll_interval ?poll_interval .				\
	OPTIONAL												\
	{															\
		gnos:map gnos:last_update ?last_update .				\
	}															\
}';

	var source = new EventSource('/query?name=primary&expr={0}&expr2={1}&expr3={2}&expr4={3}'.
		format(encodeURIComponent(expr), encodeURIComponent(expr2), encodeURIComponent(expr3), encodeURIComponent(expr4)));
	source.addEventListener('message', function(event)
	{
		GNOS.primary_data = event.data;
		redraw();
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('primary stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('primary stream closed');
		}
	});
}

function register_alerts_query()
{
	var expr = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?device ?count												\
WHERE 														\
{																\
	?device gnos:num_errors ?count							\
}';

	var source = new EventSource('/query?name=alerts&expr={0}'.
		format(encodeURIComponent(expr)));
	source.addEventListener('message', function(event)
	{
		GNOS.alert_data = {};
		var data = JSON.parse(event.data);
		for (var i=0; i < data.length; ++i)
		{
			var row = data[i];
			GNOS.alert_data[row.device] = row.count;
			console.log("row{0}: {1:j}".format(i, row));
		}
		
		if (GNOS.primary_data)
			redraw();
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('alerts stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('alerts stream closed');
		}
	});
}

function redraw()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	var data = JSON.parse(GNOS.primary_data);
	populate_devices(context, data[0], data[2]);
	draw_map(context, data[0], data[3]);
	draw_relations(context, data[1]);
}

function populate_devices(context, devices, meters)
{
	GNOS.devices = {};
	
	for (var i=0; i < devices.length; ++i)
	{
		var device = devices[i];
		GNOS.devices[device.name] =
			{
				center: new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height),
				radius: 0.0,			// set by draw_device
				stroke_width: 0.0,		// set by draw_device
				meters: []
			};
	}
	
	for (var i=0; i < meters.length; ++i)
	{
		var meter = meters[i];
		if (meter.device in GNOS.devices)
			GNOS.devices[meter.device].meters.push({label: meter.label, level: meter.level, description: meter.description});
		else
			console.log("meter {0} exists on {1} but that device doesn't exist".format(meter.label, meter.device));
	}
}

function draw_initial_map()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	var base_styles = ['primary_label'];
	var lines = ['Loading...'];
	var style_names = ['primary_label'];
	var stats = prep_center_text(context, base_styles, lines, style_names);
	center_text(context, base_styles, lines, style_names, new Point(map.width/2, map.height/2), stats);
}

function draw_relations(context, relations)
{
	var infos = find_line_infos(relations);
	var lines = [];
	for (var key in infos)
	{
		var line = draw_relation(context, infos[key]);
		lines.push(line);
	}
	
	var i = 0;
	for (var key in infos)		// do this after drawing lines so that the labels appear on top
	{
		label_relation(context, infos[key].r, lines[i], 0.3);
		if (infos[key].s)
			label_relation(context, infos[key].s, lines[i], 0.7);
		i += 1;
	}
}

// Returns object mapping src/dst device subjects to objects of the form:
//     {r: relation, broken: bool, from_arrow: arrow, to_arrow}
function find_line_infos(relations)
{
	var lines = {};
	
	var has_arrow = {stem_height: 16, base_width: 12};
	var no_arrow = {stem_height: 0, base_width: 0};
	
	for (var i=0; i < relations.length; ++i)
	{
		var relation = relations[i];
		
		var key = relation.src < relation.dst ? relation.src + "/" + relation.dst : relation.dst + "/" + relation.src;
		if (relation.type === "undirected")
		{
			// undirected: no arrows
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
		}
		else if (relation.type === "unidirectional")
		{
			// unidirectional: arrow for each relation
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: has_arrow, to_arrow: has_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else if (relation.type === "bidirectional")
		{
			// two-way bidirectional: no arrows
			// one-way bidirectional: broken (red) arrow
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, s: null, broken: true, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else
		{
			console.log("Bad relation type: " + relation.type);
		}
	}
	
	return lines;
}

// relation has
//     required fields: src, dst, type
//     optional fields: style, primary_label, secondary_label, tertiary_label
function draw_relation(context, info)
{
	//console.log("relation from {0} to {1}".format(GNOS.devices[info.r.src].center, GNOS.devices[info.r.dst].center));
	
	if ('style' in info.r)
		var style = info.r.style;
	else
		var style = 'identity';
		
	if (info.broken)
		var styles = [style, 'broken_relation'];
	else
		var styles = [style];
	
	var src = GNOS.devices[info.r.src];
	var dst = GNOS.devices[info.r.dst];
	
	var line = discs_to_line(new Disc(src.center, src.radius), new Disc(dst.center, dst.radius));
	line = line.shrink(src.stroke_width/2, dst.stroke_width/2);	// path strokes are centered on the path
	draw_line(context, styles, line, info.from_arrow, info.to_arrow);
	
	return line;
}

function label_relation(context, relation, line, p)
{
	if ('style' in relation)
		var style = relation.style;
	else
		var style = 'identity';
		
	// TODO: Should allow labels to have new lines. (We don't want to allow multiple
	// labels in the store because the joins get all whacko).
	var text = [];
	var style_names = [];
	if ('primary_label' in relation)
	{
		text.push(relation.primary_label);
		style_names.push('primary_relation');
	}
	if ('secondary_label' in relation)
	{
		text.push(relation.secondary_label);
		style_names.push('secondary_relation');
	}
	if ('tertiary_label' in relation)
	{
		text.push(relation.tertiary_label);
		style_names.push('tertiary_relation');
	}
	
	var center = line.interpolate(p);
	var base_styles = [style, 'label', 'relation_label'];
	
	var stats = prep_center_text(context, base_styles, text, style_names);
	context.clearRect(center.x - stats.max_width/2, center.y - stats.total_height/2, stats.max_width, stats.total_height);
	center_text(context, base_styles, text, style_names, center, stats);
}

function draw_map(context, devices, times)
{
	for (var i=0; i < devices.length; ++i)
	{
		var device = devices[i];
		//console.log('device{0}: {1:j}'.format(i, device));
		
		draw_device(context, device);
	}
	
	draw_map_labels(context, times);
}

function get_updated_label(last_update, poll_interval)
{
	if (!last_update)
	{
		// missing current (will happen if the modeler machine is slow or fails to respond)
		var label = "store has not been updated";
		var style_name = "error_label";
	}
	else
	{
		var last = new Date(last_update).getTime();
		var current = new Date().getTime();
		var next = last + 1000*poll_interval;
		
		var last_delta = interval_to_time(current - last);
		if (current < next)
		{
			var next_delta = interval_to_time(next - current);	
			var label = "updated {0} ago (next due in {1})".format(last_delta, next_delta);
			var style_name = "label";
		}
		else if (current < next + 60*1000)		// next will be when modeler starts grabbing new data so there will be a bit of a delay before it makes it all the way to the client
		{
			var label = "updated {0} ago (next is due)".format(last_delta);
			var style_name = "label";
		}
		else
		{
			var next_delta = interval_to_time(current - next);	
			var label = "updated {0} ago (next was due {1} ago)".format(last_delta, next_delta);
			var style_name = "error_label";
		}
	}
	
	return [label, style_name];
}

function draw_map_labels(context, times)
{
	var row = times[0];
	var labels = get_updated_label(row.last_update, row.poll_interval);
	draw_updated_label(context, labels[0], labels[1]);

	if (GNOS.alert_data)
		draw_alert_labels(context);
}

function draw_updated_label(context, label, style_name)
{
	var lines = [label];
	var style_names = [];
	style_names.push(style_name);
	
	var base_styles = ['xsmaller'];
	var stats = prep_center_text(context, base_styles, lines, style_names);
	
	var center = new Point(context.canvas.width/2, stats.total_height/2);
	center_text(context, base_styles, lines, style_names, center, stats);
}

function draw_alert_labels(context)
{
	if ('http://www.gnos.org/2012/schema#map' in GNOS.alert_data)
	{
		var count = GNOS.alert_data['http://www.gnos.org/2012/schema#map'];
		
		var lines = [];
		var style_names = [];
		if (count)
			lines.push("1 error alert");
		else
			lines.push("{0} error alerts".format(count));
		style_names.push('error_label');
		
		var base_styles = ['map'];
		var stats = prep_center_text(context, base_styles, lines, style_names);
		
		var center = new Point(context.canvas.width/2, context.canvas.height - stats.total_height/2);
		center_text(context, base_styles, lines, style_names, center, stats);
	}
}

// device has
// required fields: name, center_x, center_y
// optional fields: style, primary_label, secondary_label, tertiary_label
function draw_device(context, device)
{
	// Figure out which styles apply to the device as a whole.
	var base_styles = ['identity'];
	if ('style' in device)
		base_styles = device.style.split(' ');
	
	// Get each line of text to render and the style for that line.
	var lines = [];
	var style_names = [];
	if ('primary_label' in device)
	{
		lines.push(device.primary_label);
		style_names.push('primary_label');
	}
	if ('secondary_label' in device)
	{
		lines.push(device.secondary_label);
		style_names.push('secondary_label');
	}
	if ('tertiary_label' in device)
	{
		lines.push(device.tertiary_label);
		style_names.push('tertiary_label');
	}
	
	var next_meter = lines.length;
	for (var i=0; i < GNOS.devices[device.name].meters.length; ++i)
	{
		var meter = GNOS.devices[device.name].meters[i];
		if (meter.level >= GNOS.ok_level)				// TODO: may want an inspector option to show all meters
		{
			lines.push("{0}% {1}".format(Math.round(100*meter.level), meter.label));	// TODO: option to show description?
			style_names.push('secondary_label');
		}
	}
	
	if (GNOS.alert_data && device.name in GNOS.alert_data)
	{
		if (GNOS.alert_data[device.name] === 1)
			lines.push("1 error alert");
		else
			lines.push("{0} error alerts".format(GNOS.alert_data[device.name]));
		style_names.push('error_label');
	}
	
	// Get the dimensions of the text.
	var stats = prep_center_text(context, base_styles, lines, style_names);
	//console.log("stats: {0:j}".format(stats));
	
	// Draw a disc behind the text.
	var center = new Point(map.width * device.center_x, map.height * device.center_y);
	var radius = 1.1 * Math.max(stats.total_height, stats.max_width)/2;
	var style = draw_disc(context, base_styles, new Disc(center, radius));
	
	GNOS.devices[device.name].radius = radius;
	GNOS.devices[device.name].stroke_width = style.lineWidth;
	
	// Draw a progress bar sort of thing for each meter.
	for (var i=0; i < GNOS.devices[device.name].meters.length; ++i)
	{
		var meter = GNOS.devices[device.name].meters[i];
		if (meter.level >= GNOS.ok_level)				// TODO: may want an inspector option to show all meters
		{
			draw_meter(context, base_styles, meter, stats, center, radius, next_meter);
			next_meter += 1;
		}
	}
	
	// Draw the text.
	base_styles.push('label');
	center_text(context, base_styles, lines, style_names, center, stats);
}

function draw_meter(context, styles, meter, stats, center, radius, next_meter)
{
	context.save();
	
	if (meter.level < GNOS.ok_level)
		styles = styles.concat('good_level');
	else if (meter.level < GNOS.warn_level)
		styles = styles.concat('ok_level');
	else if (meter.level < GNOS.danger_level)
		styles = styles.concat('warn_level');
	else 
		styles = styles.concat('danger_level');
	apply_styles(context, styles);
	
	var top = center.y - stats.total_height/2;
	for (var i = 0; i < next_meter; ++i)
	{
		top += stats.heights[i];
	}
	var left = center.x - stats.widths[next_meter]/2;
	
	var height = stats.heights[next_meter];
	var width = stats.widths[next_meter];
	context.clearRect(left, top, width, height);
	
	width = meter.level * stats.widths[next_meter];
	context.fillRect(left, top, width, height);
	
	context.restore();
}
