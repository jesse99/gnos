"use strict";

window.onload = function()
{
	GNOS.store_event_ids = [];
	
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
	
	// Add place holders for each store's subjects.
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
	
	var element = document.getElementById("body");
	element.innerHTML = html;
	
	// Add sse callbacks to update the place holders.
	GNOS.store_event_ids = [];
	for (var i = 0; i < solution.length; ++i)
	{
		var store = solution[i].name;
		register_store_event(store);
	}
	
	return [];
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
	var element = document.getElementById("{0}-store".format(store));

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
	
	element.innerHTML = html;
	
	return [];
}
