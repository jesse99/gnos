use Path = path::Path;
use io::WriterUtil;
use std::json;
use std::map::*;
//use server = rwebserve;
use handlers::*;
use rrdf::*;
use task_runner::*;
use rwebserve::{Config, Request, Response, ResponseHandler, OpenSse, linear_map_from_vector, Route};

priv fn copy_scripts(root: &Path, user: &str, host: &str) -> option::Option<~str>
{
	let dir = core::os::make_absolute(root).pop();		// gnos/html => /gnos
	let dir = dir.push(~"scripts");						// /gnos => /gnos/scripts
	let files = utils::list_dir_path(&dir, ~[~".json", ~".py"]);
	
	utils::scp_files(files, user, host)
}

priv fn modeler_exited(script: &str, err: option::Option<~str>, state_chan: oldcomm::Chan<model::Msg>)
{
	let mesg =
		if err.is_some()
		{
			fmt!("%s exited with stderr: %s\n", script, err.get())
		}
		else
		{
			fmt!("%s exited with no stderr", script)
		};
	
	let lines = mesg.split_char('\n');
	for lines.each |line| {error!("%s", *line)};
	
	let alert = model::Alert {target: ~"gnos:container", id: fmt!("%s exited", script), level: ~"error", mesg: mesg, resolution: ~"Restart gnos."};	// TODO: probably should have a button somewhere to restart the script (would have to close the alert)
	oldcomm::send(state_chan, model::UpdateMsg(~"primary", |store, _err| {model::open_alert(store, &alert)}, ~""));
}

priv fn run_modeler(user: &str, host: &str, script: &str, network_file: &str, ip: &str, port: u16) -> option::Option<~str>
{
	utils::run_remote_command(user, host, fmt!("python %s --ip=%s --port=%? -v %s", script, ip, port, network_file))
}

priv fn setup(options: &options::Options, state_chan: oldcomm::Chan<model::Msg>) -> ~[ExitFn]
{
	let mut cleanup = ~[];
	
	let path = options.root.push(~"generated");			// html/generated
	if !os::path_is_dir(&path)
	{
		os::make_dir(&path, 7*8*8 + 7*8 + 7);
	}
	
	let root = copy options.root;
	let client = copy options.client_ip;
	let action: task_runner::JobFn = |copy client| copy_scripts(&root, env!("GNOS_USER"), client);
	task_runner::run_blocking(Job {action: action, policy: task_runner::ShutdownOnFailure}, ~[]);
	
	let network_file = copy(options.network_file);
	let mut modelers = ~[];
	for options.devices.each |device|
	{
		if !vec::contains(modelers, &@copy device.modeler)
		{
			let client = copy options.client_ip;
			let network_file = copy network_file;
			let modeler = copy device.modeler;
			let ip = copy options.bind_ip;
			let port = copy options.bind_port;
			let action: task_runner::JobFn = |copy client, copy modeler| run_modeler(env!("GNOS_USER"), client, modeler, network_file, ip, port);
			let script = Job {action: action, policy: task_runner::NotifyOnExit(|err, copy modeler| {modeler_exited(modeler, err, state_chan)})};
			modelers.push(@copy modeler);
			
			let exit: task_runner::ExitFn = || {utils::run_remote_command(env!("GNOS_USER"), client, fmt!("pgrep -f %s | xargs --no-run-if-empty kill -9", modeler));};
			task_runner::sequence(~[script], ~[copy exit]);
			cleanup.push(exit);
		}
	}
	
	cleanup
}

priv fn get_shutdown(cleanup: ~[ExitFn]) -> !
{
	info!("received shutdown request");
	for cleanup.each |f| {(*f)()};
	libc::exit(0)
}

priv fn update_globals(store: &Store, options: &options::Options) -> bool
{
	store.add(~"gnos:globals", ~[
		(~"gnos:admin", @BoolValue(true)),		// TODO: get this from a setting
		(~"gnos:debug", @BoolValue(true)),		// TODO: get this from command line
	]);
	
	let devices = vec::zip(vec::from_elem(options.devices.len(), ~"gnos:device"), do options.devices.map |n| {@StringValue(copy n.managed_ip, ~"")});
	store.add(~"gnos:globals", devices);
	
	let names = model::get_standard_store_names();
	let stores = vec::zip(vec::from_elem(names.len(), ~"gnos:store"), do names.map |n| {@StringValue(n.to_owned(), ~"")});
	store.add(~"gnos:globals", stores);
	
	true
}

priv fn static_view(options: &options::Options, config: &Config, request: &Request, response: Response) -> Response
{
	let mut response = rwebserve::configuration::static_view(config, request, response);
	
	// Generated images can be cached but, in general, they should expire just before we get new info.
	if request.path.starts_with("/generated/")
	{
		response.headers.insert(~"Cache-Control", fmt!("max-age=%?", options.poll_rate - 1));
	}
	response
}

fn main()
{
	info!("starting up gnos");
	if env!("GNOS_USER").is_empty()
	{
		error!("GNOS_USER must be set to the name of a user able to ssh into the network json client.");
		libc::exit(1)
	}
	
	let mut options = options::parse_command_line(os::args());
	options::validate(&options);
	
	let state_chan = do utils::spawn_moded_listener(task::ThreadPerCore) |port, copy options| {model::manage_state(port, options.bind_ip, options.bind_port)};
	let samples_chan = do utils::spawn_moded_listener(task::ThreadPerCore) |port| {samples::manage_samples(port)};
	let cleanup = if !options.db
		{
			setup(&options, state_chan)
		}
		else
		{
			db::setup(state_chan, options.poll_rate);
			~[]
		};
	
	let options1 = copy options;
	oldcomm::send(state_chan, model::UpdateMsg(~"globals", |store, _err| {update_globals(store, &options1)}, ~""));
	
	// TODO: Shouldn't need all of these damned explicit types but rustc currently
	// has problems with type inference woth closures and borrowed pointers.
	let models_v: ResponseHandler = |_config, _request, response, copy options| {get_models::get_models(&options, response, state_chan)};
	let subject_v: ResponseHandler = |_config, request, response, copy options| {get_subject::get_subject(&options, request, response)};
	let details_v: ResponseHandler = |_config, request, response, copy options| {get_details::get_details(&options, request, response)};
	let home_v: ResponseHandler = |_config, _request, response, copy options| {get_home::get_home(&options, response)};
	let modeler_p: ResponseHandler = |_config, request, response, copy options| {put_json::put_json(&options, state_chan, samples_chan, request, response)};
	let query_store_v: ResponseHandler = |_config, request, response, copy options| {get_query_store::get_query_store(&options, request, response)};
	let bail_v: ResponseHandler = |_config, _request, _response| {get_shutdown(copy cleanup)};
	let static_v: ResponseHandler = |config, request, response, copy options| {static_view(&options, config, request, response)};
	let test_v: ResponseHandler = |_config, request, response| {get_test::get_test(request, response)};
	
	let query_s: OpenSse = |_config, request, push| {sse_query::sse_query(state_chan, request, push)};
	let samples_s: OpenSse = |_config, request, push| {sse_samples::sse_query(samples_chan, request, push)};
	
	let config = Config
	{
		// We need to bind to the server addresses so that we receive modeler PUTs.
		// We bind to localhost to ensure that we can hit the web server using a local
		// browser.
		hosts: if options.db {~[~"localhost"]} else if options.admin {~[copy options.bind_ip, ~"localhost"]} else {~[copy options.bind_ip]},
		port: options.bind_port,
		server_info: ~"gnos " + options::get_version(),
		resources_root: copy options.root,
		routes: ~[
			Route(~"home", ~"GET", ~"/"),
			Route(~"details", ~"GET", ~"/details/{name}/*subject"),
			Route(~"shutdown", ~"GET", ~"/shutdown"),		// TODO: enable this via debug cfg (or maybe via a command line option)
			Route(~"models", ~"GET", ~"/models"),
			Route(~"query_store", ~"GET", ~"/query-store"),
			Route(~"subject", ~"GET", ~"/subject/{name}/*subject"),
			Route(~"test", ~"GET", ~"/test"),
			Route(~"modeler", ~"GET", ~"/modeler"),
			Route(~"modeler", ~"PUT", ~"/modeler"),
		],
		views: linear_map_from_vector(~[
			(~"home",  home_v),
			(~"details",  details_v),
			(~"shutdown",  bail_v),
			(~"models",  models_v),
			(~"query_store",  query_store_v),
			(~"subject",  subject_v),
			(~"modeler",  modeler_p),
			(~"test",  test_v),
		]),
		static_handler: static_v,
		sse: linear_map_from_vector(~[(~"/query", query_s), (~"/samples", samples_s)]),
		settings: linear_map_from_vector(~[(~"debug",  ~"true")]),		// TODO: make this a command-line option
		..rwebserve::initialize_config()
	};
	
	// There is a bit of a chicken and egg problem here: ideally we'd start up the web page after the server
	// starts but before it exits. For a while I was starting up the browser in the make file before launching
	// the server. This worked fine for a while but Chrome started timing out as the server began to do
	// more work on startup. Hopefully this will work better.
	if options.browse.is_not_empty()
	{
		let url = copy options.browse;
		do task::spawn {core::run::program_output("git", ~[~"web--browse", copy url]);}
	}
	
	rwebserve::start(&config);
	info!("exiting gnos");						// won't normally land here
}
