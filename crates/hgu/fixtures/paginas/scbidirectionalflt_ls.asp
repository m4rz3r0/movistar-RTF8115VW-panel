
<!DOCTYPE html>
<html>
    <head>
        <meta http-equiv='Pragma' CONTENT='no-cache'>
        <meta http-equiv="X-UA-Compatible" content="IE=9; IE=8; IE=7; IE=EDGE"/>
        <meta http-equiv='Content-Type' content='text/html; charset=UTF-8'>
        <link rel=stylesheet href='stylemain.css' type='text/css'>
        <link rel=stylesheet href='colors.css' type='text/css'>
        <script language="javascript" src="/js/portName.js?r=13020"></script>
        <script language="javascript" src="/js/te_util.js?r=13020"></script>
        <script language="javascript" src="/js/jquery.min.js?r=13020"></script>
        <script language="javascript" src="/js/jquery-migrate.min.js?r=13020"></script>
    <script language='javascript'>
<!-- hide

var PROTOCOL =
[
    "NONE",
    "ICMP",
    "TCP",
    "UDP"
];

var ACTION =
[
    "drop",
    "reject",
    "permit"
];

var REJECTTYPE =
[
    "none",
    "icmpv4_net_unreachable",
    "icmpv4_host_unreachable",
    "icmpv4_proto_unreachable",
    "icmpv4_port_unreachable",
    "icmpv4_net_prohibited",
    "icmpv4_host_prohibited",
    "tcp_reset",
    "icmpv6_no_route_to_destination",
    "icmpv6_administratively_prohibited",
    "icmpv6_address_unreachable",
    "icmpv6_port_unreachable",
    "icmpv6_source_address_failed",
    "icmpv6_reject_route_to_destination"
];

var ICMPTYPE =
[
    "none",
    "any",
    "echo_reply",
    "destination_unreachable",
    "network_unreachable",
    "host_unreachable",
    "protocol_unreachable",
    "port_unreachable",
    "fragmentation_needed",
    "source_route_failed",
    "network_unknown",
    "host_unknown",
    "network_prohibited",
    "host_prohibited",
    "tos_network_unreachable",
    "tos_host_unreachable",
    "communication_prohibited",
    "host_precedence_violation",
    "precedence_cutoff",
    "source_quench",
    "redirect",
    "network_redirect",
    "host_redirect",
    "tos_network_redirect",
    "tos_host_redirect",
    "echo_request",
    "router_advertisement",
    "router_solicitation",
    "time_exceeded",
    "ttl_zero_during_transit",
    "ttl_zero_during_reassembly",
    "parameter_problem",
    "ip_header_bad",
    "required_option_missing",
    "timestamp_request",
    "timestamp_reply",
    "address_mask_request",
    "address_mask_reply",
    "destination_unreachable",
    "no_route_to_destination",
    "administratively_prohibited",
    "beyond_scope_of_source_address",
    "address_unreachable",
    "port_unreachable",
    "source_address_failed",
    "reject_route_to_destination",
    "error_source_routing_header",
    "packet_too_big",
    "time_exceeded",
    "hop_limit_exceeded",
    "fragment_reassembly_time_exceeded",
    "parameter_problem",
    "erroneous_header_field",
    "unrecognized_next_header_type",
    "unrecognized_ipv6_option",
    "echo_request",
    "echo_reply",
    "multicast_listener_query",
    "multicast_listener_report",
    "multicast_listener_done",
    "router_solicitation",
    "router_advertisement",
    "neighbor_solicitation",
    "neighbor_advertisement",
    "redirect_message",
    "router_renumbering",
    "router_renumbering_command",
    "router_renumbering_result",
    "sequence_number_reset",
    "node_information_query",
    "node_information_response",
    "inverse_neighbor_discovery_solicitation",
    "inverse_neighbor_discovery_advertisement",
    "version2_multicast_listener_report",
    "multicast_router_advertisement",
    "multicast_router_solicitation",
    "multicast_router_termination"
];

var sessKey = '0000000000';
var filter = [{"FID":"0","Name":"IPv4PppIn","Interface":"ppp0","IntfID":"0.2","Type":"0","Version":"0","Default":"0","rule":[{"RID":"0","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.100","OrigMask":"255.255.255.240","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"22","DestEndPort":"22"},{"RID":"1","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.100","OrigMask":"255.255.255.240","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"443","DestEndPort":"443"},{"RID":"2","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.101","OrigMask":"255.255.255.128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"22","DestEndPort":"22"},{"RID":"3","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.101","OrigMask":"255.255.255.128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"443","DestEndPort":"443"},{"RID":"4","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.102","OrigMask":"255.255.255.0","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"22","DestEndPort":"22"},{"RID":"5","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.102","OrigMask":"255.255.255.0","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"443","DestEndPort":"443"},{"RID":"6","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.103","OrigMask":"255.255.255.0","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"22","DestEndPort":"22"},{"RID":"7","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.103","OrigMask":"255.255.255.0","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"443","DestEndPort":"443"},{"RID":"8","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"1","OrigAddr":"0.0.0.0","OrigMask":"255.255.255.255","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"0","DestEndPort":"0"},{"RID":"15","Enable":"1","Protocol":"2","Action":"2","RejectType":"0","IcmpType":"0","OrigAddr":"192.0.2.101","OrigMask":"255.255.255.128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"0.0.0.0","DestMask":"255.255.255.255","DestStartPort":"7547","DestEndPort":"7547"}]},{"FID":"1","Name":"IPv6WanIn","Interface":"WAN","IntfID":"3.-1","Type":"0","Version":"1","Default":"0","rule":[{"RID":"9","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"38","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"},{"RID":"10","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"47","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"},{"RID":"11","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"48","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"},{"RID":"12","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"51","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"},{"RID":"13","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"55","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"},{"RID":"14","Enable":"1","Protocol":"1","Action":"2","RejectType":"0","IcmpType":"56","OrigAddr":"::","OrigMask":"128","OrigStartPort":"0","OrigEndPort":"0","DestAddr":"::","DestMask":"128","DestStartPort":"0","DestEndPort":"0"}]}];
$(function() {
    if (document.referrer.search('scbidirectionalflt_ls') == 0)
      window.location.reload(); // reload when push back button on edit page

    var tbl = $("#firewall");
    var tfn = '<tr><td class="hd">Name</td>'
            + '<td class="hd" style="word-break:keep-all;">Interface</td>'
            + '<td class="hd">Type</td>'
            + '<td class="hd">IP Version</td>'
            + '<td class="hd" colspan="9">Default Action</td>'
            + '<td class="hd">Select</td></tr>';
    var trn = '<tr><td class="hd">Enabled</td>'
            + '<td class="hd">Protocol</td>'
            + '<td class="hd">Specific Action</td>'
            + '<td class="hd">Rejection Type</td>'
            + '<td class="hd">ICMP Type</td>'
            + '<td class="hd">Originator IP Address</td>'
            + '<td class="hd">Originator Mask</td>'
            + '<td class="hd">Originator Start Port</td>'
            + '<td class="hd">Originator End Port</td>'
            + '<td class="hd">Destination IP Address</td>'
            + '<td class="hd">Destination Mask</td>'
            + '<td class="hd">Destination Start Port</td>'
            + '<td class="hd">Destination End Port</td>'
            + '<td class="hd">Select</td></tr>';

    var tfo, tro;
    for (var i=0; i<filter.length; i++) {
        var type = (filter[i].Type==="1")?"OUT":"IN";
        var ver = (filter[i].Version==="1")?"IPv6":"IPv4";
        var dfl = (filter[i].Default==="1")?"permit":"drop";
        tbl.append(tfn);
        tfo = '<tr><td>' + filter[i].Name + '</td>'
            + '<td style="word-break:break-all;">' + filter[i].Interface + '</td>'
            + '<td>' + type + '</td>'
            + '<td>' + ver + '</td>'
            + '<td colspan="9"><div title="10">' + dfl + '</div></td>'
            + '<td><input type="radio" name="itemSelect" auxiliary="'
            + filter[i].Name + '|'
            + filter[i].IntfID + '|'
            + filter[i].Type + '|'
            + filter[i].Version + '|'
            + filter[i].Default
            + '" value="' + filter[i].FID + '" /></td></tr>';
        tbl.append(tfo);
        tbl.append(trn);
        for (var j=0; j<filter[i].rule.length; j++) {
            var enable = (filter[i].rule[j].Enable==="1")?"Enable":"Disable";
            var proto  = PROTOCOL[filter[i].rule[j].Protocol];
            var action = ACTION[filter[i].rule[j].Action];
            var reject = REJECTTYPE[filter[i].rule[j].RejectType];
            var icmp   = ICMPTYPE[filter[i].rule[j].IcmpType];
            var oaddr  = filter[i].rule[j].OrigAddr;
            var omask  = filter[i].rule[j].OrigMask;
            var daddr  = filter[i].rule[j].DestAddr;
            var dmask  = filter[i].rule[j].DestMask;
            var osport = filter[i].rule[j].OrigStartPort;
            var oeport = filter[i].rule[j].OrigEndPort;
            var dsport = filter[i].rule[j].DestStartPort;
            var deport = filter[i].rule[j].DestEndPort;

            oaddr  = ((oaddr==="0.0.0.0")||(oaddr==="::"))?"":oaddr;
            omask  = ((omask==="255.255.255.255")||(omask==="128"))?"":omask;
            daddr  = ((daddr==="0.0.0.0")||(daddr==="::"))?"":daddr;
            dmask  = ((dmask==="255.255.255.255")||(dmask==="128"))?"":dmask;
            osport = (osport==="0")?"":osport;
            oeport = (oeport==="0")?"":oeport;
            dsport = (dsport==="0")?"":dsport;
            deport = (deport==="0")?"":deport;

            tro = '<tr><td><div title=' + i + '>' + enable + '</div></td>'
                + '<td>' + proto + '</td>'
                + '<td>' + action + '</td>'
                + '<td>' + reject + '</td>'
                + '<td>' + icmp + '</td>'
                + '<td>' + oaddr + '</td>'
                + '<td>' + omask + '</td>'
                + '<td>' + osport + '</td>'
                + '<td>' + oeport + '</td>'
                + '<td>' + daddr + '</td>'
                + '<td>' + dmask + '</td>'
                + '<td>' + dsport + '</td>'
                + '<td>' + deport + '</td>'
                + '<td><input type="radio" name="itemSelect" auxiliary="'
                + filter[i].rule[j].Enable + '|'
                + filter[i].rule[j].Protocol + '|'
                + filter[i].rule[j].Action + '|'
                + filter[i].rule[j].RejectType + '|'
                + filter[i].rule[j].IcmpType + '|'
                + filter[i].rule[j].OrigAddr + '|'
                + filter[i].rule[j].OrigMask + '|'
                + filter[i].rule[j].OrigStartPort + '|'
                + filter[i].rule[j].OrigEndPort + '|'
                + filter[i].rule[j].DestAddr + '|'
                + filter[i].rule[j].DestMask + '|'
                + filter[i].rule[j].DestStartPort + '|'
                + filter[i].rule[j].DestEndPort
                + '" value="' + filter[i].FID + '|' + filter[i].rule[j].RID + '" /></td></tr>';
            tbl.append(tro);
        }
    }
});

function persistState( idx, aux, duration ) {
   var d=new Date();
   if( duration>0 ) d.setTime(d.getTime()+(duration*60*60*1000));
   else d.setTime(d.getTime()-1);
   var expires="expires="+d.toGMTString();
   document.cookie="indexToBeEdited="+idx+";"+expires+";"+"path=/";
   document.cookie="auxiliaryToBeEdited="+aux+";"+expires+";"+"path=/";
}

function addClick() {
   persistState('', '', 0);

   var loc = 'scbidirectionalflt_ed.asp';

   var code = 'location="' + loc + '"';
   eval(code);
}

function editClick(item) {
   var selectedItem = '';
   var selectedAuxiliary = '';
   if (item.length > 0)
      for (i = 0; i < item.length; i++) {
         if ( item[i].checked == true ) {
            selectedItem = item[i].value;
            selectedAuxiliary = item[i].getAttribute("auxiliary");
            break;
         }
      }
   else if ( item.checked == true )
      selectedItem = item.value;

   if( selectedItem!='' && selectedAuxiliary!='' )
   {
      persistState(selectedItem, selectedAuxiliary, 6);
      var loc = 'scbidirectionalflt_ed.asp';

      var code = 'location="' + loc + '"';
      eval(code);
   }
   else
   {
      alert('Please select an item!');
   }
}

function removeClick(item, escape) {
   var selectedItem = '';
   if (item.length > 0)
      for (i = 0; i < item.length; i++) {
         if ( item[i].checked == true ) {
            selectedItem = item[i].value;
            break;
         }
      }
   else if ( item.checked == true )
      selectedItem = item.value;
   //alert(selectedItem);
   var loc = '/cgi-bin/scbidirectionalflt.cmd?action=remove&item=' + selectedItem;

   if(escape==true) loc += '&escape=true';

   loc += '&sessionKey='+sessKey;
   var code = 'location="' + loc + '"';
   eval(code);
}

var ismobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
function changemenu(){
  if(ismobile){
    $('#escapeDoor').find('img:first').attr('width','22');
    $('#escapeDoor').find('img:first').attr('height','21');
    $('#escapeDoor').find('img:last').attr('width','99');
    $('#escapeDoor').find('img:last').attr('height','28');
    $('#escapeDoor').find('img:last').attr('src','te_menu_micon.jpg');
    $('form[style="padding:15px;"]').find('table').wrap('<div style="overflow:auto;"></div>');
  }
}

// done hiding -->
</script>
</head>
<body onload="changemenu();" class="logoBody" topmargin="0" leftmargin="0">
<div style="height:80px;" id="escapeDoor">
  <div style="position:absolute;margin-left:10%;width:20%;left:10px;height:80px;display:inline-block;">
    <div style="position:relative;"><img style="float:left;" src="te_logo.jpg"></div>
    <div style="position:relative;">
    <a href="index.asp" target="mainFrame" title="Back to Web Client">
    <img style="float:left;cursor:pointer;z-index:1;" src="te_menu_icon.jpg">
    </a>
    </div>
  </div>
  <div style="position:absolute;margin-right:10%;width:30%;right:10px;height:80px;display:inline-block;">
     <div style="position:relative;line-height:60px;text-align:right;font-family:'Arial','toTahoma','Helvetica','sans-serif';font-size:28px;color:#5BC500;">Router Smart WiFi</div>
  </div>
</div>
<div>
<form style="padding:15px;">
<b>Incoming/Outgoing IP Filtering Setup</b><br><br>
By default, all incoming/outgoing IP traffic is droped/passed, but some IP traffic can be <b><font color='red'>BLOCKED</font><font color='blue'>&#47;</font><font color='green'>ACCEPTED</font></b> by setting up filters.<br><br>
Choose Add or Remove or Edit to configure incoming/outgoing IP filters.<br><br>
<center>
<table id='firewall' border='1' cellpadding='4' cellspacing='0' width="100%">
</table><br>
<input type='button' onClick='addClick()' value='Add'>
<input type='button' onClick='removeClick(this.form.itemSelect, true)' value='Remove'>
<input type='button' onClick='editClick(this.form.itemSelect)' value='Edit'>
</center>
</form>
</div>
</body>
</html>
