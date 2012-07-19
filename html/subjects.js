"use strict";

window.onload = function()
{
	var expr = '											\
PREFIX 													\
	gnos: <http://www.gnos.org/2012/schema#>	\
SELECT DISTINCT 										\
	?name 												\
WHERE 													\
{ 															\
	?subject ?predicate ?object . 						\
	BIND(rrdf:pname(?subject) AS ?name) 			\
} ORDER BY ?name';
	var source = new EventSource('/query?name=model&expr='+encodeURIComponent(expr));
	source.addEventListener('message', function(event)
	{
		var table = document.getElementById('subjects');
		var data = JSON.parse(event.data);
		
		var html = '';
		for (var i=0; i < data.length; ++i)
		{
			var row = data[i];
			var klass = i & 1 ? "odd" : "even";
			html += '<tr class="{0}"><td class="value"><span>\
							<a href="/subject/{1}">{2}</a>\
						</span></td></tr>\n'.format(
							klass, encodeURIComponent(row.name), escapeHtml(row.name));
		}
		table.innerHTML = html;
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('> subjects stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('> subjects stream closed');
		}
	});
}
