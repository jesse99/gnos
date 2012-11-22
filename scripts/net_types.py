# Types used by snmp and linux_ssh to communicate with net-modeler.
class Interface(object):
	def __init__(self):
		self.admin_ip = None		# "10.14.0.2"
		self.index = None			# "2"
		self.name = None			# "eth1"
		self.status = None			# "up", "down", "dormant", etc
		self.ip = None				# "10.14.0.254"			may not be set if the device is inactive
		self.net_mask = None		# "255.255.255.0"			may not be set if the device is inactive
		self.mac_addr = None		# "00:19:bb:5f:59:8a"		may not be set if the device is inactive
		self.speed = None			# 10000000.0 bps
		self.mtu = None			# 1500 bytes
		self.in_octets = None		# 9840.0 bytes
		self.out_octets = None		# 9840.0 bytes
		self.last_changed = None	# 2191.0 seconds
		
	# True if the interface is able to communicate.
	@property
	def active(self):
		return self.status == 'up' or self.status == 'dormant'
	
	def __repr__(self):
		return self.ip or '?'

class Link(object):
	def __init__(self):
		self.admin_ip = None
		self.predicate = None
		self.peer_ip = None
		self.label1 = None			# may include stuff like age or cost
		self.label2 = None
		self.label3 = None
	
	def __repr__(self):
		return self.peer_ip or '?'

class Route(object):
	def __init__(self):
		self.via_ip = None
		self.dst_subnet = None
		self.dst_mask = None
		self.protocol = None
		self.metric = None
		self.ifindex = None		# source interface index
		
		self.src_interface = None
		self.via_interface = None
		self.dst_admin_ip = None
	
	def __repr__(self):
		return '%s via %s' % (self.dst_subnet or '?', self.via_ip or '?')

class MRoute(object):
	def __init__(self):
		self.admin_ip = None
		self.group = None			# ip
		self.source = None		# device ip (may be zero)
		self.upstream = None		# device ip
		self.protocol = None
		self.uptime = None
		self.label1 = None			# relation detail 1
		self.label2 = None			# relation detail 2
		self.label3 = None			# relation detail 3
		
		self.packets = None		# float
		self.octets = None			# float
	
	def __repr__(self):
		return '%s/%s from %s' % (self.group or '?', self.source or '?', self.upstream or '?')

class Device(object):
	def __init__(self, name, config):
		self.__name = name		# from network json
		self.__config = config		# from network json
		
		self.uptime = None		# 60.0 seconds
		self.system_info = ''		# "markdown"
		self.interfaces = []			# [Interface]
		self.links = []				# [Link]
		self.routes = []				# [Route]
		self.mroutes = []			# [MRoute]
		self.pim_hellos = {}		# {ifindex => seconds}
		self.ospf_hellos = {}		# {device ip => seconds}
		self.ospf_deads = {}		# {device ip => seconds}
	
	@property
	def name(self):
		return self.__name
	
	@property
	def config(self):
		return self.__config
	
	@property
	def admin_ip(self):
		return self.__config['ip']
		
	def find_ifindex(self, ifindex):
		for interface in self.interfaces:
			if interface.index == ifindex:
				return interface
		return None
		
	def find_ip(self, ip):
		for interface in self.interfaces:
			if interface.ip == ip:
				return interface
		return None
		
	def __repr__(self):
		return self.__config['ip']
