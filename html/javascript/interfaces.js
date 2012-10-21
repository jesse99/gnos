// Used to view interface bandwidth details for a device.
"use strict";

$(document).ready(function()
{
	var body = document.getElementById('body');
	var owner = body.getAttribute("data-owner");
	register_query(owner);
});

function register_query(owner)
{
	var source = new EventSource('/samples?owner={0}'.format(encodeURIComponent(owner)));
	GNOS.sse_query = {source: source, owner: owner};
	GNOS.update_count = 0;
	
	var chart = document.getElementById('chart');
	GNOS.base_src = chart.src;
	
	var body = document.getElementById('body');
	source.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		animated_draw(body, function () {do_update(data);});
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

// TODO: Should be able to toggle between the chart and a table via a click.
// Chart could perhaps contain timestamps.
function do_update(details)
{
	// See http://www.post-hipster.com/2008/10/20/using-javascript-to-refresh-an-image-without-a-cache-busting-parameter/
	GNOS.update_count += 1;
	var chart = document.getElementById('chart');
	chart.src = "{0}#{1}".format(GNOS.base_src, GNOS.update_count);
	
	var html = "";
	html += "<table border='1'>\n";
	html += "	<tr>\n";
	html += "		<th>Interface</th>\n";
	html += "		<th>Min</th>\n";
	html += "		<th>Mean</th>\n";
	html += "		<th>Max</th>\n";
	html += "	</tr>\n";
	for (var i = 0; i < details.length; ++i)
	{
		var detail = details[i];
		
		// These are formatted as "10.102.0.2-eth0-in-octets".
		var parts = detail.sample_name.split('-');
		var name = parts[1];
		
		var units = escapeHtml(detail.units.replace("b/s", "bps"));
		
		html += "	<tr>\n";
		html += "		<td>{0}</td>\n".format(escapeHtml(name));
		html += "		<td>{0} {1}</td>\n".format(detail.min.toFixed(1), units);	// TODO: maybe include a timestamp for these?
		html += "		<td>{0} {1}</td>\n".format(detail.mean.toFixed(1), units);
		html += "		<td>{0} {1}</td>\n".format(detail.max.toFixed(1), units);
		html += "	</tr>\n";
	}
	html += "</table>\n";
	
	var stats = document.getElementById('stats');
	stats.innerHTML = html;
}
