"use strict";

window.onload = function()
{
	GNOS.models = {}
	
	var expr = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 											\
	?name 													\
WHERE 														\
{ 																\
	gnos:globals gnos:store ?name . 						\
} ORDER BY ?name';
	var source = new EventSource('/query?name=globals&expr={0}'.
		format(encodeURIComponent(expr)));
	source.addEventListener('message', function(event)
	{
		var body = document.getElementById('body');
		var data = JSON.parse(event.data);
		
		var html = "";
		for (var i = 0; i < data.length; ++i)
		{
			var row = data[i];
			
			html += '<h2>{0}</h2>\n'.format(escapeHtml(row.name));
			html += '<table border="1" class="model" id="{0}-store">\n'.format(row.name);
			html += '</table>\n';
			
			register_store_event(row.name);
		}
		body.innerHTML = html;
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('models> globals stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('models> globals stream closed');
		}
	});
}

function register_store_event(name)
{
	if (name in GNOS.models)
		GNOS.models[name].close();
	
	var expr = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 											\
	?name 													\
WHERE 														\
{ 																\
	?subject ?predicate ?object . 							\
	BIND(rrdf:pname(?subject) AS ?name) 				\
} ORDER BY ?name';
	var source = new EventSource('/query?name={0}&expr={1}'.
		format(name, encodeURIComponent(expr)));
	source.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		
		var element = document.getElementById('body');
		animated_draw(element, function() {update_html(name, data);});
		
	});
	GNOS.models[name] = source;
	
	source.addEventListener('open', function(event)
	{
		console.log('models> {0} stream opened'.format(name));
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('models> {0} stream closed'.format(name));
		}
	});
}

function update_html(name, data)
{
	var html = "";
	for (var i = 0; i < data.length; ++i)
	{
		var row = data[i];
		
		var klass = i & 1 ? "odd" : "even";
		html += '<tr class="{0}"><td class="value"><span>\
						<a href="/subject/{3}/{1}">{2}</a>\
					</span></td></tr>\n'.format(
						klass, encodeURIComponent(row.name), escapeHtml(row.name), encodeURIComponent(name));
	}
	
	var body = document.getElementById("{0}-store".format(name));
	body.innerHTML = html;
}
