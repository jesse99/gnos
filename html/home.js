"use strict";

function updateHeaders(delta)
{
	var d = new Date();
	var days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
	
	var header = document.getElementById('h{0}'.format(delta));
	if (delta == 0)
	{
		header.innerHTML = days[d.getDay()] + " (today)";
	}
	else
	{
		d.setDate(d.getDate() - delta);
		header.innerHTML = days[d.getDay()];
	}
}

function updateAlerts(delta, data)
{
	var right = new Date();
	right.setDate(right.getDate() - delta);
	
	var left = new Date();
	left.setDate(left.getDate() - (delta + 1));
	
	var html = "";
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		console.log('row{0}: {1}', i, row);
		
		var time = new Date(row.date);
		if (time > left && time <= right)
		{
			html += '<li><span class="{0}">{1}</span></li>\n'.format(
				row.level, escapeHtml(row.mesg));
		}
	}
	
	if (!html)
	{
		html = '<li><span class="no-error">No alerts</span></li>\n';
	}
	
	var list = document.getElementById('l{0}'.format(delta));
	list.innerHTML = html;
}

function updateState(data)
{
	for (var i=0; i < 4; ++i)
	{
		updateHeaders(i);
		updateAlerts(i, data);
	}
}

window.onload = function()
{
	updateState([]);
	
	var oldest = new Date();
	oldest.setDate(oldest.getDate() - 4);
	var expr = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?mesg ?level ?date											\
WHERE 															\
{																	\
	?subject gnos:alert ?mesg .								\
	?subject gnos:level ?level .									\
	?subject gnos:timestamp ?date .							\
	FILTER (?date >= "{0}"^^xsd:dateTime)				\
} ORDER BY ?date ?mesg'.format(oldest.toISOString());

	var source = new EventSource('/query?name=alerts&expr='+encodeURIComponent(expr));
	source.addEventListener('message', function(event)
	{
	console.log('got alert data');
		var data = JSON.parse(event.data);
		updateState(data);
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('> alerts stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('> alerts stream closed');
		}
	});
}
