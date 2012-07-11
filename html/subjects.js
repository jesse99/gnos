// TODO:
// get this working with subjects.html
// create the EventSource and see what happens at the server
// close the EventSource and see what happens at the server

window.onload = function()
{
	console.log("loaded page");
	
	window.source = new EventSource('/query');
	window.source.addEventListener('message', function(event)
	{
		console.log(event.data);
	});
	
	window.source.addEventListener('open', function(event)
	{
		console.log('> Connection was opened');
	});
	
	window.source.addEventListener('error', function(event)
	{
		if (event.eventPhase == 2)
		{
			console.log('> Connection was closed');
		}
	});
}
