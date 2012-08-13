// Immutable Point, Size, Rect, etc classes.
// All coordinates are screen coordinates.
// These are immutable in that the properties they define are fixed, but new mutable
// properties can be added to any of these.
"use strict";

// ---- Point class -----------------------------------------------------------
function Point(x, y)
{
	this.x = x;
	this.y = y;
	freezeProps(this);
}

Point.zero = new Point(0, 0);

// Returns the distance between this and rhs.
Point.prototype.distance = function (rhs)
{
	var dx = this.x - rhs.x;
	var dy = this.y - rhs.y;
	return Math.sqrt(dx*dx + dy*dy);
}

Point.prototype.distance_squared = function (rhs)
{
	var dx = this.x - rhs.x;
	var dy = this.y - rhs.y;
	return dx*dx + dy*dy;
}

Point.prototype.toString = function ()
{
	return "{x: " + this.x + ", y: " + this.y + "}";
}

// ---- Size class -----------------------------------------------------------
function Size(width, height)
{
	this.width = width;
	this.height = height;
	freezeProps(this);
}

Size.zero = new Size(0, 0);

Size.prototype.toString = function ()
{
	return "{width: " + this.width + ", height: " + this.height + "}";
}

// ---- Rect class ------------------------------------------------------------
function Rect(topLeft, size)
{
	this.topLeft = topLeft;
	this.size = size;
	freezeProps(this);
}

Rect.prototype.toString = function ()
{
	return "{left: " + this.topLeft.x  + ", top: " + this.topLeft.y + ", width: " + this.size.width + ", height: " + this.size.height + "}";
}

// ---- Line class ------------------------------------------------------------
function Line(from, to)
{
	this.from = from;
	this.to = to;
	freezeProps(this);
}

Line.prototype.normals = function ()
{
	var unit = this.normalize();
	return [new Point(-unit.y, unit.x), new Point(unit.y, -unit.x)];
}

// Returns the line as a unit vector.
Line.prototype.normalize = function ()
{
	var x = this.to.x - this.from.x;
	var y = this.to.y - this.from.y;
	var d = this.to.distance(this.from);
	return new Point(x/d, y/d);
}

// Returns a point on the line from this.from (0.0) to this.to (1.0).
Line.prototype.interpolate = function (p)
{
	assert(p >= 0.0, "position is negative");
	assert(p <= 1.0, "position is larger than 1.0");
	
	var dx = this.to.x - this.from.x;
	var dy = this.to.y - this.from.y;
	
	return new Point(this.from.x + p*dx, this.from.y + p*dy);
}

// Shrinks the line by from_delta pixels on the from side and to_delta pixels on the to side.
Line.prototype.shrink = function (from_delta, to_delta)
{
	var theta = Math.atan((this.to.y - this.from.y)/(this.to.x - this.from.x));
	
	var dx = from_delta * Math.cos(theta);
	var dy = from_delta * Math.sin(theta);
	var from = new Point(this.from.x + dx, this.from.y + dy);
		
	dx = to_delta * Math.cos(theta);
	dy = to_delta * Math.sin(theta);
	var to = new Point(this.to.x + dx, this.to.y + dy);
	
	return new Line(from, to);
}

Line.prototype.toString = function ()
{
	return "{from: " + this.from + ", to: " + this.to + "}";
}

// ---- Disc class ------------------------------------------------------------
function Disc(center, radius)
{
	this.center = center;
	this.radius = radius;
	freezeProps(this);
}

// Returns true if this intersects the rhs disc.
Disc.prototype.intersects = function (rhs)
{
	return this.center.distance(rhs.center) <= this.radius || rhs.center.distance(this.center) <= rhs.radius;
}

// Returns true if this intersects the rhs Point.
Disc.prototype.intersects_pt = function (rhs)
{
	return this.center.distance(rhs) <= this.radius;
}

// Returns the point on this perimeter closest to pt.
Disc.prototype.intersection = function (pt)
{
	var dx = pt.x - this.center.x;
	var dy = pt.y - this.center.y;
	var theta = Math.atan(dy/dx);
	
	var x = this.center.x + this.radius * Math.cos(theta);
	var y = this.center.y + this.radius * Math.sin(theta);
	var pt1 = new Point(x, y);
	
	x = this.center.x - this.radius * Math.cos(theta);
	y = this.center.y - this.radius * Math.sin(theta);
	var pt2 = new Point(x, y);
	
	if (pt1.distance(pt) < pt2.distance(pt))
		return pt1;
	else
		return pt2;
}

Disc.prototype.toString = function ()
{
	return "{center: " + this.center + ", radius: " + this.radius + "}";
}
