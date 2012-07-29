"use strict";

// Maps object names ("_:obj1") to objects of the form:
// {
//    x: 0.3
//    y: 0.6
// }
var objects = {};

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
	?tertiary_label ?type										\
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
		populate_objects(data[0]);
		draw_relations(context, data[1]);
		draw_map(context, data[0]);
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

function populate_objects(data)
{
	objects = {};
	
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		
		objects[row.object] = 
		{
			x: row.center_x,
			y: row.center_y
		};
	}
}

function draw_initial_map()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	context.fillStyle = 'cornflowerblue';
	
	center_text(context, ['xlarger'], ['Loading...'], ['primary_label'], map.width/2, map.height/2);
}

function draw_relations(context, data)
{
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		//console.log('row{0}: {1:j}'.format(i, row));
		
		draw_relation(context, row);
	}
}

// object has
// required fields: src, dst, type
// optional fields: primary_label, secondary_label, tertiary_label
function draw_relation(context, object)
{
	context.save();
	console.log("relation from {0:j} to {1:j}".format(objects[object.src], objects[object.dst]));
	draw_line(context, objects[object.src], objects[object.dst]);
	
	context.restore();
}

function draw_map(context, data)
{
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		//console.log('row{0}: {1}'.format(i, JSON.stringify(row)));
		
		draw_object(context, row);
	}
}

// object has
// required fields: object, center_x, center_y
// optional fields: style, primary_label, secondary_label, tertiary_label
function draw_object(context, object)
{
	context.save();
	
	var style_names = ['identity'];
	if ('style' in object)
		style_names = compose_styles(object.style.split(' '));
	context.fillStyle = 'black';
	
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
	
	if (lines)
	{
		var x = map.width * object.center_x;
		var y = map.height * object.center_y;
		center_text(context, style_names, lines, style_names, x, y);
	}
	context.restore();
}

