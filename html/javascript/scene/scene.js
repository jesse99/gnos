// Mutable class used to manipulate, draw, and hit test a list of shape objects.
"use strict";

function Scene()
{
	this.shapes = [];
}

Scene.prototype.append = function (shape)
{
	this.shapes.push(shape);
}

Scene.prototype.find = function (predicate)
{
	for (var i = 0; i < this.shapes.length; ++i)
	{
		if (predicate(this.shapes[i]))
			return this.shapes[i];
	}
	
	return null;
}

Scene.prototype.remove_all = function ()
{
	this.shapes = [];
}

Scene.prototype.remove_if = function (predicate)
{
	this.shapes = this.shapes.filter(
		function(shape)
		{
			return !predicate(shape);
		});
}

Scene.prototype.draw = function (context)
{
	for (var i = 0; i < this.shapes.length; ++i)
	{
		//context.save();
		this.shapes[i].draw(context);
		//context.restore();
	}
}

Scene.prototype.hit_test = function (pt)
{
	// Iterate backwards so that the first shapes that respond to
	// clicks are the shapes that appear on top.
	for (var i = this.shapes.length - 1; i >= 0; --i)
	{
		if (this.shapes[i].hit_test(pt))
			return this.shapes[i];
	}
	
	return null;
}

Scene.prototype.toString = function ()
{
	return "Scene with {0} shapes".format(this.shapes.length);
}
