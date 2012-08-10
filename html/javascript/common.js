"use strict";

// We tuck away all of our global variables into this object to minimize the
// risk of name clashes with other libraries or new versions of javascript.
var GNOS = {}

// Replaces {0} with argument 0, {1} with argument 1, etc.
// Argument index can be appended with ":j" to print the argument as json.
String.prototype.format = function()
{
	var args = arguments;
	return this.replace(/{(\d+)(:j)?}/g,
		function(match, number, json)
		{
			if (json)
				return typeof args[number] !== 'undefined' ? JSON.stringify(args[number]) : 'undefined';
			else
				return typeof args[number] !== 'undefined' ? args[number] : 'undefined';
		}
	);
};

function AssertException(message)
{
	this.message = message;
}

AssertException.prototype.toString = function ()
{
	return 'assert: ' + this.message;
}

function assert(predicate, message)
{
	if (!predicate)
		throw new AssertException(message);
}

function clone(obj)
{
	return JSON.parse(JSON.stringify(obj));	
}

// Make the all (or the explicitly named) properties of object nonwritable and nonconfigurable. 
// Based on a similar function from JavaScript: The Definitive Guide.
function freezeProps(object /*, names*/)
{
	var props = arguments.length === 1
		? Object.getOwnPropertyNames(object) : Array.prototype.splice.call(arguments, 1);
		
	// Make each configurable property read-only and permanent.
	props.forEach(function (name)
	{
		if (Object.getOwnPropertyDescriptor(object, name).configurable)
			Object.defineProperty(object, name, {writable: false, configurable: false});
	});
	
	return object;
}

function escapeHtml(str)
{
	var div = document.createElement('div');
	div.appendChild(document.createTextNode(str));
	return div.innerHTML;
};

// Converts an interval in milliseconds to a string like "2 seconds" or "1.2 hours".
function interval_to_time(interval)
{
	if (interval < 1000)
	{
		var value = interval.toFixed();
		var units = "millisecond";
	}
	else if (interval < 60*1000)
	{
		var value = (interval/1000).toFixed();
		var units = "second";
	}
	else if (interval < 60*60*1000)
	{
		var value = (interval/(60*1000)).toFixed(1);
		var units = "minute";
	}
	else if (interval < 60*60*60*1000)
	{
		var value = (interval/(60*60*1000)).toFixed(1);
		var units = "hour";
	}
	else
	{
		var value = (interval/(24*60*60*1000)).toFixed(2);
		var units = "day";
	}
	
	if (value !== '1')		// note that we don't want to say 1.0 minute
		return value + " " + units + "s";
	else
		return value + " " + units;
}
