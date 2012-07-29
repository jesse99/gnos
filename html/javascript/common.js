"use strict";

// Replaces {0} with argument 0, {1} with argument 1, etc.
// Argument index can be appended with ":j" to print the argument as json.
String.prototype.format = function()
{
	var args = arguments;
	return this.replace(/{(\d+)(:j)?}/g,
		function(match, number, json)
		{
			if (json)
				return typeof args[number] != 'undefined' ? JSON.stringify(args[number]) : 'undefined';
			else
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

function clone(obj)
{
	return JSON.parse(JSON.stringify(obj));	
}

