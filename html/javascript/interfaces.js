// Used to view interface bandwidth details for a device.
"use strict";

window.onload = function()
{
	var body = document.getElementById('body');
	var owner = body.getAttribute("data-owner");
	register_query(owner);
};

function register_query(owner)
{
console.log("registering samples query for {0}".format(owner));
	
	var source = new EventSource('/samples?owner={0}'.format(encodeURIComponent(owner)));
	GNOS.sse_query = {source: source, owner: owner};
	GNOS.update_count = 0;
	
	var chart = document.getElementById('chart');
	GNOS.base_src = chart.src;
	
	source.addEventListener('message', function(event)
	{
console.log("samples update: {0}".format(event.data));
		var data = JSON.parse(event.data);
		do_update(data);
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('samples> {0} opened'.format(owner));
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
			console.log('samples> {0} closed'.format(owner));
		else
			console.log('samples> {0} error: {1}'.format(owner, event.eventPhase));
	});
}

function do_update(details)
{
	// See http://www.post-hipster.com/2008/10/20/using-javascript-to-refresh-an-image-without-a-cache-busting-parameter/
	GNOS.update_count += 1;
	var chart = document.getElementById('chart');
	chart.src = "{0}#{1}".format(GNOS.base_src, GNOS.update_count);
	
	var html = "";
	html += "<table border='1'>\n";
	html += "	<tr>Interface</tr>\n";
	html += "	<tr>Min</tr>\n";
	html += "	<tr>Mean</tr>\n";
	html += "	<tr>Max</tr>\n";
	for (var i = 0; i < details.length; ++i)
	{
		var detail = details[i];
		
		html += "	<tr>{0}</tr>\n".format(escapeHtml(detail.sample_name));
		html += "	<tr>{0} {1}</tr>\n".format(escapeHtml(detail.min), escapeHtml(detail.units));
		html += "	<tr>{0} {1}</tr>\n".format(escapeHtml(detail.mean), escapeHtml(detail.units));
		html += "	<tr>{0} {1}</tr>\n".format(escapeHtml(detail.max), escapeHtml(detail.units));
	}
	html += "</table>\n";
	
	var stats = document.getElementById('stats');
	stats.innerHTML = html;
}
