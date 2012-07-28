"use strict";

// Replaces {0} with argument 0, {1} with argument 1, etc.
String.prototype.format = function()
{
	var args = arguments;
	return this.replace(/{(\d+)}/g,
		function(match, number)
		{ 
			return typeof args[number] != 'undefined' ? args[number] : 'undefined';
		}
	);
};

function escapeHtml(str)
{
	var div = document.createElement('div');
	div.appendChild(document.createTextNode(str));
	return div.innerHTML;
};

