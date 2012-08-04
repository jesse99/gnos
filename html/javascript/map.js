"use strict";

// Maps device names ("_:device1") to objects of the form:
// {
//    center: Point
//    radius: Number
//    stroke_width: Number
//    meters: [{label: String, level: Number, description: String}]
// }
var DEVICES = {};

// Thresholds for different meter levels.
var GOOD_LEVEL		= 0.0;
var OK_LEVEL			= 0.5;
var WARN_LEVEL		= 0.7;
var DANGER_LEVEL	= 0.8;

window.onload = function()
{
	draw_initial_map();
	register_query();
}

function register_query()
{
	// It's rather awkward to have all these OPTIONAL clauses, but according
	// to the spec the entire OPTIONAL block must match in order to affect 
	// the solution.
	var expr = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?center_x ?center_y ?primary_label ?secondary_label	\
	?tertiary_label ?style ?name								\
WHERE 															\
{																	\
	?name gnos:center_x ?center_x .							\
	?name gnos:center_y ?center_y .							\
	OPTIONAL													\
	{																\
		?name gnos:style ?style .								\
	}																\
	OPTIONAL													\
	{																\
		?name gnos:primary_label ?primary_label .			\
	}																\
	OPTIONAL													\
	{																\
		?name gnos:secondary_label ?secondary_label .	\
	}																\
	OPTIONAL													\
	{																\
		?name gnos:tertiary_label ?tertiary_label .			\
	}																\
}';

	var expr2 = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?src ?dst ?primary_label ?secondary_label				\
	?tertiary_label ?type ?style								\
WHERE 															\
{																	\
	?rel gnos:src ?src .											\
	?rel gnos:dst ?dst .											\
	?rel gnos:type ?type .										\
	OPTIONAL													\
	{																\
		?rel gnos:style ?style .									\
	}																\
	OPTIONAL													\
	{																\
		?rel gnos:primary_label ?primary_label .				\
	}																\
	OPTIONAL													\
	{																\
		?rel gnos:secondary_label ?secondary_label .		\
	}																\
	OPTIONAL													\
	{																\
		?rel gnos:tertiary_label ?tertiary_label .				\
	}																\
}';

	var expr3 = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?label ?device ?level ?description							\
WHERE 															\
{																	\
	?indicator gnos:meter ?label .								\
	?indicator gnos:target ?device .							\
	?indicator gnos:level ?level .								\
	OPTIONAL													\
	{																\
		?indicator gnos:description ?description .			\
	}																\
}';

	var source = new EventSource('/query?name=primary&expr={0}&expr2={1}&expr3={2}'.
		format(encodeURIComponent(expr), encodeURIComponent(expr2), encodeURIComponent(expr3)));
	source.addEventListener('message', function(event)
	{
		var map = document.getElementById('map');
		var context = map.getContext('2d');
		context.clearRect(0, 0, map.width, map.height);
		
		var data = JSON.parse(event.data);
		populate_devices(context, data[0], data[2]);
		draw_map(context, data[0]);
		draw_relations(context, data[1]);
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('map stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('map stream closed');
		}
	});
}

function populate_devices(context, devices, meters)
{
	DEVICES = {};
	
	for (var i=0; i < devices.length; ++i)
	{
		var device = devices[i];
		DEVICES[device.name] = 
			{
				center: new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height),
				radius: 0.0,				// set by draw_device
				stroke_width: 0.0,		// set by draw_device
				meters: []
			};
	}
	
	for (var i=0; i < meters.length; ++i)
	{
		var meter = meters[i];
		if (meter.device in DEVICES)
			DEVICES[meter.device].meters.push({label: meter.label, level: meter.level, description: meter.description});
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
		if (relation.type == "undirected")
		{
			// undirected: no arrows
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
		}
		else if (relation.type == "unidirectional")
		{
			// unidirectional: arrow for each relation
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: has_arrow, to_arrow: has_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else if (relation.type == "bidirectional")
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
	//console.log("relation from {0} to {1}".format(DEVICES[info.r.src].center, DEVICES[info.r.dst].center));
	
	if ('style' in info.r)
		var style = info.r.style;
	else
		var style = 'identity';
		
	if (info.broken)
		var styles = [style, 'broken_relation'];
	else
		var styles = [style];
	
	var src = DEVICES[info.r.src];
	var dst = DEVICES[info.r.dst];
	
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

function draw_map(context, devices)
{
	for (var i=0; i < devices.length; ++i)
	{
		var device = devices[i];
		//console.log('device{0}: {1:j}'.format(i, device));
		
		draw_device(context, device);
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
	for (var i=0; i < DEVICES[device.name].meters.length; ++i)
	{
		var meter = DEVICES[device.name].meters[i];
		if (meter.level >= OK_LEVEL)				// TODO: may want an inspector option to show all meters
		{
			lines.push("{0}% {1}".format(Math.round(100*meter.level), meter.label));	// TODO: option to show description?
			style_names.push('secondary_label');
		}
	}
	
	// Get the dimensions of the text.
	var stats = prep_center_text(context, base_styles, lines, style_names);
	console.log("stats: {0:j}".format(stats));
	
	// Draw a disc behind the text.
	var center = new Point(map.width * device.center_x, map.height * device.center_y);
	var radius = 1.1 * Math.max(stats.total_height, stats.max_width)/2;
	var style = draw_disc(context, base_styles, new Disc(center, radius));
	
	DEVICES[device.name].radius = radius;
	DEVICES[device.name].stroke_width = style.lineWidth;
	
	// Draw a progress bar sort of thing for each meter.
	for (var i=0; i < DEVICES[device.name].meters.length; ++i)
	{
		var meter = DEVICES[device.name].meters[i];
		if (meter.level >= OK_LEVEL)				// TODO: may want an inspector option to show all meters
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
	
	if (meter.level < OK_LEVEL)
		styles = styles.concat('good_level');
	else if (meter.level < WARN_LEVEL)
		styles = styles.concat('ok_level');
	else if (meter.level < DANGER_LEVEL)
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
