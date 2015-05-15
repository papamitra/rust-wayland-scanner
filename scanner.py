#!/usr/bin/env python3

import sys
import xml.etree.ElementTree as etree
import argparse

conv_type = {
    'int': 'i32',
    'uint': 'u32',
    'fixed': 'wl_fixed_t',
    'fd': 'i32',
    'string': '&str'
}

def create_param(arg):
    n = arg.attrib['name']
    t = arg.attrib['type']
    if t == 'object':
        t = '*const ' + 'Struct_' + arg.attrib['interface']
    elif conv_type.get(t) is not None:
        t = conv_type[t]

    return n + ': ' + t

def request_dtor(elem, ifname):
    fn_name = ifname + '_' + elem.attrib['name']
    st_name = 'Struct_' + ifname
    arg = ifname + ': *mut ' + st_name
    fn = 'pub fn {0}({1}) -> ()'.format(fn_name, arg)

    fn_def = """{{
    unsafe {{
        wl_proxy_destroy({0} as *mut Struct_wl_proxy);
    }}
}}""".format(ifname)

    print(fn,)
    print(fn_def)
    print()

def request(elem, ifname):
    if elem.attrib.get('type') == 'destructor':
        request_dtor(elem, ifname)
        return
    name = elem.attrib['name']
    st_name = 'Struct_' + ifname
    params = [ifname + ': *mut ' + st_name]
    args = []
    ret_type = '*mut ::libc::c_void'
    is_return = False
    ret_name = 'id'
    ex_lines = []
    for arg in elem.findall('arg'):
        t = arg.attrib['type']
        if t == 'new_id':
            is_return = True
            if arg.attrib.get('interface') is None:
                params.append('interface: *const Struct_wl_interface')
                params.append('version: u32')
                args.append('(*interface).name')
                args.append('version')
                args.insert(0, 'interface')
            else:
                ret_type = '*mut ' + 'Struct_' + arg.attrib['interface']
                args.insert(0, '&{0}_interface'.format(arg.attrib['interface']))
            args.append('::std::ptr::null::<::libc::c_void>()')
            ret_name = arg.attrib['name']

            continue
        elif t == 'string':
            ex_lines.append('let {0}_c_string = std::ffi::CString::new({0}).unwrap();'.format(arg.attrib['name']))
            args.append(arg.attrib['name'] + '_c_string.as_ptr()')
        else:
            args.append(arg.attrib['name'])
        params.append(create_param(arg))

    fn_name = ifname + '_' + name
    if is_return:
        fn = 'pub fn {0}({1}) -> {2}'.format(fn_name,
                                             ', '.join(params),
                                             ret_type)

        fn_def = """{{
    let {0}: *mut Struct_wl_proxy;
    {5}
    unsafe {{
        {0} = wl_proxy_marshal_constructor({1} as *mut Struct_wl_proxy, {2}, {3});
    }}
    {0} as {4}
}}""".format(ret_name, ifname, fn_name.upper(), ', '.join(args), ret_type, '\n'.join(ex_lines))
    else:
        fn = 'pub fn {0}({1}) -> ()'.format(fn_name,
                                             ', '.join(params))

        fn_def = """{{
    {3}
    unsafe {{
        wl_proxy_marshal({0} as *mut Struct_wl_proxy, {1}, {2});
    }}
}}""".format(ifname, fn_name.upper(), ', '.join(args), '\n'.join(ex_lines))

    print(fn,)
    print(fn_def)
    print()

def iface_header(tree):
    name = tree.attrib['name']
    for i,req in enumerate(tree.findall('request')):
        enum_name = (name + '_' + req.attrib['name']).upper()
        print("const {0}: ::libc::c_uint = {1};".format(enum_name, i))
    #print("extern {{ static {0}_interface: Struct_wl_interface;}}".format(name))
    print()

def add_listener(ifname):
    print("""
pub fn {0}_add_listener({0}: *mut Struct_{0},
                        listener: *const Struct_{0}_listener,
                        data: *mut ::libc::c_void) -> ::libc::c_int {{
    unsafe {{
        return wl_proxy_add_listener({0} as *mut Struct_wl_proxy,
                                     listener as *mut std::option::Option<extern "C" fn()->()>, data);
    }}
}}
""".format(ifname))

def interface(tree):
    iface_header(tree)

    name = tree.attrib['name']
    for e in tree:
        if e.tag == 'request':
            request(e, name)

    if tree.findall('event'):
        add_listener(name)

def protocol(tree):
    for i in tree.findall('interface'):
        interface(i)

def header():
    print("""
use client::wayland_client::*;
//include!("./wayland_client.rs");
use std;
""")

def usage():
    print("""usage:
scanner.py /path/to/wayland.xml
""")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('input', help='path to protocol.xml')
    parser.add_argument('-o', '--output', help='path to output rust file')
    args = parser.parse_args()

    stdout = sys.stdout
    try:
        if args.output:
            sys.stdout = open(args.output,'w')

        header()

        with open(args.input) as f:
            xmlstr = f.read()
            root = etree.fromstring(xmlstr)
            protocol(root)
    finally:
        if sys.stdout != stdout:
            close(sys.stdout)
            sys.stdout = stdout

if __name__ == '__main__':
    main()
