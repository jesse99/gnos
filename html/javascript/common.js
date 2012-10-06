// Misc utility functions.
"use strict";

// We tuck away all of our global variables into this object to minimize the
// risk of name clashes with other libraries or new versions of javascript.
var GNOS = {};

Array.prototype.intersect = function(rhs)
{
	var result = [];
	
	for (var i = 0; i < this.length; ++i)
	{
		if (rhs.indexOf(this[i]) >= 0)
			result.push(this[i]);
	}
	
	return result;
};

Array.prototype.intersects = function(rhs)
{
	for (var i = 0; i < this.length; ++i)
	{
		if (rhs.indexOf(this[i]) >= 0)
			return true;
	}
	
	return false;
};

Array.prototype.push_all = function(rhs)
{
	for (var i = 0; i < rhs.length; ++i)
	{
		this.push(rhs[i]);
	}
};

// Replaces {0} with argument 0, {1} with argument 1, etc.
// Argument index can be appended with ":j" to print the argument as json.
String.prototype.format = function()
{
	var args = arguments;
	return this.replace(/{(\d+?)(:j)?}/g,
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
};

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
	var props = arguments.length === 1 ? Object.getOwnPropertyNames(object) : Array.prototype.splice.call(arguments, 1);
		
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
}

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

// Returns [x, y] for the absolute position of an element on the page,
// From http://www.quirksmode.org/js/findpos.html
function findPos(obj)
{
	var curleft = 0;
	var curtop = 0;
	
	if (obj.offsetParent)
	{
		do
		{
			curleft += obj.offsetLeft;
			curtop += obj.offsetTop;
			obj = obj.offsetParent;
		}
		while (obj);
	}
	
	return [curleft, curtop];
}

// Finds the position of an element relative to the viewport.
// From http://blog.stannard.net.au/2010/05/22/find-the-position-of-an-element-with-javascript/
function findPosRelativeToViewport(obj)
{
	var objPos = findPos(obj);
	var scroll = getPageScroll();
	return [objPos[0] - scroll[0], objPos[1] - scroll[1]];
}

// getPageScroll() by quirksmode.org
// Finds the scroll position of a page
function getPageScroll()
{
	var xScroll, yScroll;
	if (self.pageYOffset)
	{
		yScroll = self.pageYOffset;
		xScroll = self.pageXOffset;
	} 
	else if (document.documentElement && document.documentElement.scrollTop)
	{
		yScroll = document.documentElement.scrollTop;
		xScroll = document.documentElement.scrollLeft;
	} 
	else if (document.body)	// all other Explorers
	{
		yScroll = document.body.scrollTop;
		xScroll = document.body.scrollLeft;
	}
	return [xScroll, yScroll];
}

// Fades the element out, calls render, and fades in.
// TODO: replace this with jquery fadeOut and fadeIn
function animated_draw(element, render)
{
	var timeout = 600;		// ms (this is the slow jquery speed)
	var steps = 20;			// 10 in each direction
	
	var interpolate = function(p, min, max)
	{
		assert(min <= max, "min is larger than max");
		
		p = Math.max(p, 0.0);
		p = Math.min(p, 1.0);
		return min + p*(max - min);
	};
	
	var max = parseFloat(element.style.opacity) || 1.0;
	var min = 0.1;
	if (min < max)
	{
		var p = 0.99;
		var delta = -(max - min)/steps;
		assert(delta < 0.0, "delta has to be negative or fade won't terminate");
		
		var fade = window.setInterval(function ()
		{
			var opacity = interpolate(p, min, max);
			element.style.opacity = opacity;
			
			p += delta;
			if (p <= 0.0)
			{
				render();
				delta = -delta;
			}
			else if (p >= 1.0)
			{
				element.style.opacity = max;
				window.clearInterval(fade);
			}
		}, timeout/steps);
	}
}

