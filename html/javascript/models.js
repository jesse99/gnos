"use strict";

window.onload = function()
{
	GNOS.store_event_ids = [];
	GNOS.store_updater_ids = [];
	
	register_updater("models", ["models"], "body", models_updater);
	
	var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 													\
WHERE 													\
{ 															\
	gnos:globals gnos:store ?name . 						\
} ORDER BY ?name';
	register_event("models", "globals", [query], models_handler);
}

function models_handler(solution)
{
	deregister_store_events();
	deregister_store_updaters();
	
	GNOS.model.models = solution.map(
		function (row)
		{
			return row.name;
		});
	
	return ["models"];
}

function models_updater(element, model_names)
{
	assert(model_names.length == 1, "expected one model_names but found " + model_names.length);
	
	// Add place holders for each store's subjects.
	var html = "";
	for (var i = 0; i < GNOS.model.models.length; ++i)
	{
		var store = GNOS.model.models[i];
		
		html += '<details open="open">\n';
		html += '<summary>{0}</summary>\n'.format(escapeHtml(store));
		html += '<table border="1" class="model" id="{0}-store">\n'.format(store);
		html += '</table>\n';
		html += '</details>\n';
		html += '<br>\n';
	}
	element.innerHTML = html;
	
	// Add sse callbacks to update the place holders.
	GNOS.store_event_ids = [];
	GNOS.store_updater_ids = [];
	for (var i = 0; i < GNOS.model.models.length; ++i)
	{
		var store = GNOS.model.models[i];
		
		var id = store + "-updater";
		var model_name = store + "-store";
		var element = "{0}-store".format(store);
		register_updater(id, [model_name], element, function (element, model_names) {store_updater(store, element, model_names)});
		GNOS.store_updater_ids.push(id);
		
		register_store_event(store);
	}
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

function deregister_store_updaters()
{
	for (var i = 0; i < GNOS.store_updater_ids.length; ++i)
	{
		var id = GNOS.store_updater_ids[i];
		delete GNOS.sse_updaters[i];
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
