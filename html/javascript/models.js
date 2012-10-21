// Page that shows subjects in each store.
"use strict";

$(document).ready(function()
{
console.log("registering");
	GNOS.store_query_ids = [];
	GNOS.store_renderer_ids = [];
	
	var query = '							\
SELECT DISTINCT 						\
	?name 									\
WHERE 									\
{ 											\
	gnos:globals gnos:store ?name . 		\
} ORDER BY ?name';
	register_query("models", ["models"], "globals", [query], [models_query]);
	register_renderer("models", ["models"], "body", models_renderer);
});

function models_query(solution)
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
	
	return {models: [html, solution]};
}

function models_renderer(element, model, model_names)
{
	element.html(model.models[0]);
	
	// Add sse callbacks to update the place holders.
	deregister_store_queries();
	deregister_store_renderers();
	
	var solution = model.models[1];
	for (var i = 0; i < solution.length; ++i)
	{
		var store = solution[i].name;
		register_store_query(store);
		
		var name = "{0}-store".format(store);
		register_renderer(name, [name], name, store_renderer);
		GNOS.store_renderer_ids.push(name);
	}
}

function deregister_store_queries()
{
	for (var i = 0; i < GNOS.store_query_ids.length; ++i)
	{
		var id = GNOS.store_query_ids[i];
		deregister_query(id);
	}
	GNOS.store_query_ids = [];
}

function deregister_store_renderers()
{
	for (var i = 0; i < GNOS.store_renderer_ids.length; ++i)
	{
		var id = GNOS.store_renderer_ids[i];
		deregister_renderer(id);
	}
	GNOS.store_renderer_ids = [];
}

// TODO: Chrome version 21 only supports four outstanding EventSources so snmp doesn't
// show up (if you use the Network tab in the developer panel you'll see that the snmp request
// is marked pending). The Sep 2012 beta, version 22, supports more so the snmp items do 
// show up there.
function register_store_query(store)
{
	var query = '									\
SELECT DISTINCT 								\
	?name 											\
WHERE 											\
{ 													\
	?subject ?predicate ?object . 					\
	BIND(rrdf:pname(?subject) AS ?name) 		\
} ORDER BY ?name';
	
	var id = "models-" + store;
	var model_name = "{0}-store".format(store);
	register_query(id, [model_name], store, [query], [function (solution) {return store_query(store, solution);}]);
	GNOS.store_query_ids.push(id);
}

function store_query(store, solution)
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
	
	var result = {};
	result[model_name] = html;
	return result;
}

function store_renderer(element, model, model_names)
{
	var model_name = model_names[0];
	
	element.html(model[model_name]);
}
