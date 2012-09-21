"use strict";

window.onload = function()
{
	var expr = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 													\
WHERE 													\
{ 															\
	gnos:globals gnos:store ?name . 						\
} ORDER BY ?name';
	var source = new EventSource('/query?name=globals&expr={0}'.
		format(encodeURIComponent(expr)));
	source.addEventListener('message', function(event)
	{
		var body = document.getElementById('body');
		var data = JSON.parse(event.data);
		
		var html = "";
		deregister_updaters();
		for (var i = 0; i < data.length; ++i)
		{
			var row = data[i];
			
			html += '<details open="open">\n';
			html += '<summary>{0}</summary>\n'.format(escapeHtml(row.name));
			html += '<table border="1" class="model" id="{0}-store">\n'.format(row.name);
			html += '</table>\n';
			html += '</details>\n';
			html += '<br>\n';
			
			var id = row.name + "-updater";
			var model_name = row.name + "-store";
			var element = "{0}-store".format(row.name);
			register_updater(id, [model_name], element, function (element, model_names) {store_updater(row.name, element, model_names)});
			
			store_event(row.name);
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

// TODO: Chrome version 21 only supports four outstanding EventSources so snmp doesn't
// show up (if you use the Network tab in the developer panel you'll see that the snmp request
// is marked pending). The Sep 2012 beta, version 22, supports more so the snmp items show
// up there.
function store_event(store)
{
	var id = "models-" + store;
	deregister_event(id);
	
	var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 													\
WHERE 													\
{ 															\
	?subject ?predicate ?object . 							\
	BIND(rrdf:pname(?subject) AS ?name) 				\
} ORDER BY ?name';
	register_event(id, store, [query], function (solution) {return store_handler(store, solution)});
}

function store_handler(store, solution)
{
	var names = solution.map(
		function (row)
		{
			return row.name;
		});
	
	var model_name = store + "-store";
	GNOS.model[model_name] = names;
	
	return [store + "-store"];
}

function store_updater(store, element, model_names)
{
	assert(model_names.length == 1, "expected one model_names but found " + model_names.length);
	
	var model = GNOS.model[model_names[0]];
	
	var html = "";
	for (var i = 0; i < model.length; ++i)
	{
		var klass = i & 1 ? "odd" : "even";
		html += '<tr class="{0}"><td class="value"><span>\
						<a href="/subject/{3}/{1}">{2}</a>\
					</span></td></tr>\n'.format(
						klass, encodeURIComponent(model[i]), escapeHtml(model[i]), encodeURIComponent(store));
	}
	
	element.innerHTML = html;
}
