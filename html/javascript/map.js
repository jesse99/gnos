"use strict";

window.onload = function()
{
	draw_initial_map();
	register_query();
}

function register_query()
{
	// TODO: once we fix rrdf we should be able to use a single OPTIONAL block
	var expr = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?center_x ?center_y ?primary_label ?secondary_label	\
	?tertiary_label												\
WHERE 															\
{																	\
	gnos:map gnos:object ?object .							\
	?object gnos:center_x ?center_x .						\
	?object gnos:center_y ?center_y .						\
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

	var source = new EventSource('/query?name=model&expr='+encodeURIComponent(expr));
	source.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		draw_map(data);
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

function draw_initial_map()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	context.fillStyle = 'cornflowerblue';
	
	center_text(context, 'default_object', ['Loading...'], ['primary_label'], map.width/2, map.height/2);
}

function draw_map(data)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	context.clearRect(0, 0, map.width, map.height);
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		console.log('row{0}: {1}'.format(i, JSON.stringify(row)));
		
		draw_object(context, row);
	}
}

// object has
// required fields: center_x, center_y
// optional fields: primary_label, secondary_label, tertiary_label
function draw_object(context, object)
{
	context.save();
	
	var style_name = 'default_object';	// TODO: use style if set
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
		center_text(context, style_name, lines, style_names, x, y);
	}
	context.restore();
}

