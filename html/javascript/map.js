"use strict";

// Maps object names ("_:obj1") to objects of the form:
// {
//    center: Point
//    radius: Number
// }
var object_info = {};

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
	?tertiary_label ?style ?object								\
WHERE 															\
{																	\
	?object gnos:center_x ?center_x .						\
	?object gnos:center_y ?center_y .						\
	OPTIONAL													\
	{																\
		?object gnos:style ?style .								\
	}																\
	OPTIONAL													\
	{																\
		?object gnos:primary_label ?primary_label .		\
	}																\
	OPTIONAL													\
	{																\
		?object gnos:secondary_label ?secondary_label .	\
	}																\
	OPTIONAL													\
	{																\
		?object gnos:tertiary_label ?tertiary_label .			\
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

	var source = new EventSource('/query?name=model&expr={0}&expr2={1}'.
		format(encodeURIComponent(expr), encodeURIComponent(expr2)));
	source.addEventListener('message', function(event)
	{
		var map = document.getElementById('map');
		var context = map.getContext('2d');
		context.clearRect(0, 0, map.width, map.height);
		
		var data = JSON.parse(event.data);
		populate_objects(context, data[0]);
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

function populate_objects(context, objects)
{
	object_info = {};
	
	for (var i=0; i < objects.length; ++i)
	{
		var object = objects[i];
		object_info[object.object] = {center: new Point(object.center_x * context.canvas.width, object.center_y * context.canvas.height)};
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
	var lines = find_lines(relations);
	for (var key in lines)
	{
		draw_relation(context, lines[key]);
	}
}

// Returns object mapping src/dst object subjects to objects of the form:
//     {r: relation, broken: bool, from_arrow: arrow, to_arrow}
function find_lines(relations)
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
			// no arrows
			lines[key] = {r: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
		}
		else if (relation.type == "unidirectional")
		{
			// arrow for each relation
			if (key in lines)
				lines[key] = {r: relation, broken: false, from_arrow: has_arrow, to_arrow: has_arrow};
			else
				lines[key] = {r: relation, broken: false, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else if (relation.type == "bidirectional")
		{
			// no arrows unless the relation is one way
			if (key in lines)
				lines[key] = {r: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, broken: true, from_arrow: no_arrow, to_arrow: has_arrow};
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
	console.log("relation from {0} to {1}".format(object_info[info.r.src].center, object_info[info.r.dst].center));
	
	if ('style' in info.r)
		var style = info.r.style;
	else
		var style = 'identity';
		
	if (info.broken)
		var styles = [style, 'broken_relation'];
	else
		var styles = [style];
	
	var src = object_info[info.r.src];
	var dst = object_info[info.r.dst];
	
	var line = discs_to_line(new Disc(src.center, src.radius), new Disc(dst.center, dst.radius));
	line = line.shrink(src.stroke_width/2, dst.stroke_width/2);	// path strokes are centered on the path
	draw_line(context, styles, line, info.from_arrow, info.to_arrow);
}

function draw_map(context, objects)
{
	for (var i=0; i < objects.length; ++i)
	{
		var object = objects[i];
		//console.log('object{0}: {1:j}'.format(i, object));
		
		draw_object(context, object);
	}
}

// object has
// required fields: object, center_x, center_y
// optional fields: style, primary_label, secondary_label, tertiary_label
function draw_object(context, object)
{
	// Figure out which styles apply to the object as a whole.
	var base_styles = ['identity'];
	if ('style' in object)
		base_styles = object.style.split(' ');
	
	// Get each line of text to render and the style for that line.
	var lines = [];
	var style_names = [];
	if ('primary_label' in object)
	{
		lines.push(object.primary_label);
		style_names.push('primary_label');
	}
	if ('secondary_label' in object)
	{
		lines.push(object.secondary_label);
		style_names.push('secondary_label');
	}
	if ('tertiary_label' in object)
	{
		lines.push(object.tertiary_label);
		style_names.push('tertiary_label');
	}
	
	// Get the dimensions of the text.
	var stats = prep_center_text(context, base_styles, lines, style_names);
	console.log("stats: {0:j}".format(stats));
	
	// Draw a disc behind the text.
	var center = new Point(map.width * object.center_x, map.height * object.center_y);
	var radius = 1.1 * Math.max(stats.total_height, stats.max_width)/2;
	var style = draw_disc(context, base_styles, new Disc(center, radius));
	
	object_info[object.object].radius = radius;
	object_info[object.object].stroke_width = style.lineWidth;
	
	// Draw the text.
	base_styles.push('label');
	center_text(context, base_styles, lines, style_names, center, stats);
}

