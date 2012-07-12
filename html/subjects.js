"use strict";

// Replaces {0} with argument 0, {1} with argument 1, etc.
String.prototype.format = function()
{
	var args = arguments;
	return this.replace(/{(\d+)}/g,
		function(match, number)
		{ 
			return typeof args[number] != 'undefined' ? args[number] : match;
		}
	);
};

window.onload = function()
{
	var source = new EventSource('/query');
	source.addEventListener('message', function(event)
	{
		var table = document.getElementById('subjects');
		var data = JSON.parse(event.data);
		
		var html = '';
		for (var i=0; i < data.length; ++i)
		{
			var row = data[i];
			var klass = i & 1 ? "odd" : "even";
			html += '<tr class="{0}"><td class="value"><span><a href="/subject/{1}">{1}</a></span></td></tr>\n'.format(klass, row.name);
		}
		table.innerHTML = html;
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('> stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('> stream closed');
		}
	});
}
