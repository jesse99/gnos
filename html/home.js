"use strict";

// Returns a string like "Wednesday 18:06".
function dateToStr(date)
{
	if (date.getHours() < 10)
	{
		var prefix = '0';
	}
	else
	{
		var prefix = '';
	}
	
	var days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
	return "{0} {1}:{2}".format(days[date.getDay()], prefix+date.getHours(), date.getMinutes());
}

function updateAlerts(data, kind)
{
	var html = "";
	for (var i=0; i < data.length; ++i)
	{
		var row = data[i];
		//console.log('row{0}: {1}', i, row);
		
		if ((kind == 'active' && !('end' in row)) || (kind != 'active' && 'end' in row))
		{
			if ('end' in row)
			{
				var date = new Date(row.end);
			}
			else
			{
				var date = new Date(row.begin);
			}
			
			html += '<li class="{0}-{1}" title="{2}">{3} ({4})</li>\n'.format(
				kind, row.level, escapeHtml(row.resolution), escapeHtml(row.mesg), dateToStr(date));
		}
	}
	
	if (!html)
	{
		html = '<li class="{0}-no-alerts">None</li>\n'.format(kind);
	}
	
	var list = document.getElementById('{0}-list'.format(kind));
	list.innerHTML = html;
}

window.onload = function()
{
	updateAlerts([], "active");
	updateAlerts([], "inactive");
	
	var oldest = new Date();
	oldest.setDate(oldest.getDate() - 4);
	var expr = '													\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 															\
	?mesg ?resolution ?level ?begin ?end					\
WHERE 															\
{																	\
	?subject gnos:mesg ?mesg .								\
	?subject gnos:level ?level .									\
	?subject gnos:begin ?begin .								\
	?subject gnos:resolution ?resolution .					\
	OPTIONAL													\
	{																\
		?subject gnos:end ?end								\
	}																\
	FILTER (?begin >= "{0}"^^xsd:dateTime)				\
} ORDER BY ?begin ?mesg'.format(oldest.toISOString());

	var source = new EventSource('/query?name=alerts&expr='+encodeURIComponent(expr));
	source.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		updateAlerts(data, "active");
		updateAlerts(data, "inactive");
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
