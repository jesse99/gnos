"use strict";

window.onload = function()
{
	GNOS.store_event_ids = [];
	GNOS.store_updater_ids = [];
	
	var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 													\
WHERE 													\
{ 															\
	gnos:globals gnos:store ?name . 						\
} ORDER BY ?name';
	register_event("models", "globals", [query], models_handler);
	register_updater("models", ["models"], "body", models_updater);
}

function models_handler(solution)
{
	var html = "";
	for (var i = 0; i < solution.length; ++i)
	{
		var store = solution[i].name;
		
		html += '<details open="open">\n';
		html += '<summary>{0}</summary>\n'.format(escapeHtml(store));
		html += '<table border="1" class="model" id="{0}-store">\n'.format(store);
		html += '</table>\n';
		html += '</details>\n';
		html += '<br>\n';
	}
	
	GNOS.model.models = [html, solution];
	
	return ["models"];
}

function models_updater(element, model_names)
{
	element.innerHTML = GNOS.model.models[0];
	
	// Add sse callbacks to update the place holders.
	deregister_store_events();
	deregister_updater_events();
	
	var solution = GNOS.model.models[1];
	for (var i = 0; i < solution.length; ++i)
	{
		var store = solution[i].name;
		register_store_event(store);
		
		var name = "{0}-store".format(store);
		var element = document.getElementById(name);
		register_updater(name, [name], name, store_updater);
		GNOS.store_updater_ids.push(name);
	}
	
	GNOS.model.models = null;
}

function deregister_store_events()
{
	for (var i = 0; i < GNOS.store_event_ids.length; ++i)
	{
		var id = GNOS.store_event_ids[i];
		delete GNOS.sse_events[i];
	}
	GNOS.store_event_ids = [];
}

function deregister_updater_events()
{
	for (var i = 0; i < GNOS.store_updater_ids.length; ++i)
	{
		var id = GNOS.store_updater_ids[i];
		deregister_updater(id);
	}
	GNOS.store_updater_ids = [];
}

// TODO: Chrome version 21 only supports four outstanding EventSources so snmp doesn't
// show up (if you use the Network tab in the developer panel you'll see that the snmp request
// is marked pending). The Sep 2012 beta, version 22, supports more so the snmp items do 
// show up there.
function register_store_event(store)
{
	var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 													\
WHERE 													\
{ 															\
	?subject ?predicate ?object . 							\
	BIND(rrdf:pname(?subject) AS ?name) 				\
} ORDER BY ?name';
	
	var id = "models-" + store;
	register_event(id, store, [query], function (solution) {return store_handler(store, solution)});
	GNOS.store_event_ids.push(id);
}

function store_handler(store, solution)
{
	var model_name = "{0}-store".format(store);
	
	var html = "";
	for (var i = 0; i < solution.length; ++i)
	{
		var name = solution[i].name;
		
		var klass = i & 1 ? "odd" : "even";
		html += '<tr class="{0}"><td class="value"><span>\
						<a href="/subject/{3}/{1}">{2}</a>\
					</span></td></tr>\n'.format(
						klass, encodeURIComponent(name), escapeHtml(name), encodeURIComponent(store));
	}
	
	GNOS.model[model_name] = html;
	
	return [model_name];
}

function store_updater(element, model_names)
{
	var model_name = model_names[0];
	
	element.innerHTML = GNOS.model[model_name];
	GNOS.model[model_name] = null;
}
