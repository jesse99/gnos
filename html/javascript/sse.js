// Used to simplify dynamic web views.
//
// Based on three core concepts:
// 1) An event which contains the name of a store and SPARQL query(s) against that store.
// 2) A handler which processes solutions for an event pushed put by the server and updates the
// GNOS.model dictionary.
// 3) An updater which is called when a particular model value changes.
//
// Events and handlers are often swapped in and out depending upon the current selection.
// Updaters tend to be more static.
"use strict";

// Adds store query(s) + a function used to update GNOS.model.
// id is an arbitrary string used to remove the event+handler
// store is the name of a serve store, e.g. "primary"
// query is a SPARQL query which the server will continuously run
// handler is a function which takes solution(s) and returns the names of the GNOS.model keys 
//     which were updated
function register_event(id, store, queries, handler)
{
	if (!GNOS.sse_events)
		GNOS.sse_events = {};
	assert(!GNOS.sse_events[id], id + " is already a registered event");
//console.log("registering {0} event for {1}".format(id, store));
	
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
		var data = JSON.parse(event.data);
//console.log('sse> {0} received update'.format(id));
		
		if (!GNOS.model)
			GNOS.model = {};
		var model_names = handler(data);
//console.log('sse> model_names = {0:j}'.format(model_names));
		if (model_names)
		{
//console.log('sse> {0} updating'.format(id));
			do_model_change(model_names);
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
	
	GNOS.sse_events[id] = source;
}

// Removes an existing query.
function deregister_event(id)
{
//console.log("deregistering {0} event".format(id));
	if (GNOS.sse_events)
		delete GNOS.sse_events[id];
}

// Add a function which is called when a named GNOS.model value changes.
// id is an arbitrary string used to remove the updater
// model_names contains a list of model names as returned by handlers
// element_name is the id of the html element which will be redrawn
// updater is a function taking the element and model names which changed and returning nothing
function register_updater(id, model_names, element_name, updater)
{
	if (!GNOS.sse_updaters)
		GNOS.sse_updaters = {};
	assert(!GNOS.sse_updaters[id], id + " is already a registered updater");
//console.log("registering {0} updater for {1:j}".format(id, model_names));
	
	GNOS.sse_updaters[id] =
		{
			models: model_names,
			element: element_name,
			callback: updater,
		};
}

// Removes an existing updater.
function deregister_updater(id)
{
//console.log("deregistering {0} updater".format(id));
	if (GNOS.sse_updaters)
	{
		delete GNOS.sse_updaters[id];
	}
}

// Removes all updaters.
function deregister_updaters()
{
//console.log("deregistering all updaters");
	delete GNOS.sse_updaters;
}

// ---- Internal Functions --------------------------------------------------------------
function do_model_change(model_names)
{
	// Figure out which updaters need to be called.
	var ids = [];
	for (var id in GNOS.sse_updaters)
	{
		var updater = GNOS.sse_updaters[id];
		if (updater.models.some(function (name) {return model_names.indexOf(name) >= 0}))
			ids.push(id);
	}
	
	// Call them.
	for (var i = 0; i < ids.length; ++i)
	{
		var updater = GNOS.sse_updaters[ids[i]];
		var element = document.getElementById(updater.element);
		animated_draw(element, function () {updater.callback(element, model_names)});
	}
}
