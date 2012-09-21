// Helpers used to manage queries and updating for dynamic views.
//
// The design consists of a client-side model which is populated with query functions and viewed 
// with renderer callbacks. This provides a layer of indirection between queries and views which
// allows the queries to change without affecting the views.
"use strict";

// Adds SPARQL query(s) used to update the model. Renderers will be automatically called
// as the model changes.
//
// id is an arbitrary string used with deregister_query
// model_names is a list of strings
// store is the name of a server store, e.g. "primary"
// queries is a list of SPARQL queries which the server will continuously run
// callback is of the form: function (solution(s)) -> object
//    where the object can only have fields within model_names
function register_query(id, model_names, store, queries, callback)
{
	if (!GNOS.sse_queries)
		GNOS.sse_queries = {};
	assert(!GNOS.sse_queries[id], id + " is already a registered query");
//console.log("registering {0} query for {1}".format(id, store));
	
	// Models should be associated with only one query.
	for (var qid in GNOS.sse_queries)
	{
		var candidate = GNOS.sse_queries[qid];
		var common = model_names.intersect(candidate.model_names);
		assert(common.length == 0, "{0:j} was found in {1}".format(common, qid));
	}
	
	// Start the model off in a well known state.
	if (!GNOS.sse_model)
		GNOS.sse_model = {};
	model_names.forEach(
		function (model_name)
		{
			GNOS.sse_model[model_name] = null;
		});
	
	// Create an EventSource for the query.
	var expressions = queries.map(
		function (query, index)
		{
			if (index == 0)
				return "expr={0}".format(encodeURIComponent(query));
			else
				return "expr{0}={1}".format(index+1, encodeURIComponent(query));
		});
	var source = new EventSource('/query?name={0}&{1}'.
		format(encodeURIComponent(store), expressions.join("&")));
	
	source.addEventListener('message', function(event)
	{
		// For one query this will be a solution. For multiple queries a list of solutions.
		var data = JSON.parse(event.data);
		
		var result = callback(data);
		if (result)
		{
			for (var name in result)
			{
				assert(model_names.indexOf(name) >= 0, "{0} returned {1} which is not in {2:j}".format(id, name, model_names));
				GNOS.sse_model[name] = result[name]
			}
			
			do_model_changed(Object.keys(result));
		}
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('sse> {0} opened'.format(id));
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
			console.log('sse> {0} closed'.format(id));
		else
			console.log('sse> {0} error: {1}'.format(id, event.eventPhase));
	});
	
	// Remember the details for this query.
	GNOS.sse_queries[id] = {source: source, model_names: model_names};
}

// Removes an existing query.
function deregister_query(id)
{
	var query = GNOS.sse_queries[id];
	if (query)
	{
		// Renderers will often hang around so don't leave stale state hanging around.
		query.model_names.forEach(
			function (name)
			{
				GNOS.sse_model[model_name] = null;
			});
		
		// Deterministically close the sse session.
		query.source.close();
		
		// Delete the query.
		delete GNOS.sse_queries[id];
	}
}

// Add a function which is called when an associated model object changes.
//
// id is an arbitrary string used to remove the renderer
// model_names contains a list of model names used by the renderer
// element_id is the id of the html element which will be redrawn (via an animation)
// callback is of the form: function (element, model, model_names) -> ()
//    element is the HTML element associated with the renderer
//    model is an object whose entries include the names from model_names
//       (entries not in model_names may have null values)
//    model_names are the models that have changed
function register_renderer(id, model_names, element_id, callback)
{
	if (!GNOS.sse_renderers)
		GNOS.sse_renderers = {};
	assert(!GNOS.sse_renderers[id], id + " is already a registered renderer");
//console.log("registering {0} updater for {1:j}".format(id, model_names));
	
	GNOS.sse_renderers[id] =
		{
			model_names: model_names,
			element_id: element_id,
			callback: callback,
		};
}

// Removes an existing updater.
function deregister_renderer(id)
{
	if (GNOS.sse_renderers)
	{
		delete GNOS.sse_renderers[id];
	}
}

// ---- Internal Functions --------------------------------------------------------------
function do_model_changed(model_names)
{
	for (var id in GNOS.sse_renderers)
	{
		var candidate = GNOS.sse_renderers[id];
		if (candidate.model_names.intersects(model_names))
		{
			var element = document.getElementById(candidate.element_id);
			animated_draw(element, 
				function ()
				{
					candidate.callback(element, GNOS.sse_model, model_names)
				});
		}
	}
}
