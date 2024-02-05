
mod all_to_silk;
mod deal_flac;
mod deal_silk;
mod mp3_deal;
mod wav_deal;

use std::{ffi::{c_char, c_int, CStr, CString}, os::raw::c_void, str::FromStr, sync::Mutex};

lazy_static::lazy_static! {
    static ref G_AC:Mutex<c_int>  = Mutex::new(0);
}

// #[warn(dead_code)]
// fn get_ac() -> c_int {
//     *G_AC.lock().unwrap()
// }

pub fn parse_bin(bin_data: & str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let err_str = "can't get bin type";
    if !bin_data.starts_with("12331549-6D26-68A5-E192-5EBE9A6EB998") {
        return Err(err_str.into());
    }
    let tp = bin_data.get(36..37).ok_or(err_str)?;
    if tp != "B" {
        return Err(err_str.into());
    }
    let content_text = bin_data.get(37..).ok_or(err_str)?.as_bytes();
    if content_text.len() % 2 != 0 {
        return Err(err_str.into());
    }
    let mut content2:Vec<u8> = vec![];
    for pos in 0..(content_text.len() / 2) {
        let mut ch1 = content_text[pos * 2];
        let mut ch2 = content_text[pos * 2 + 1];
        if ch1 < 0x3A {
            ch1 -= 0x30;
        }else{
            ch1 -= 0x41;
            ch1 += 10;
        }
        if ch2 < 0x3A {
            ch2 -= 0x30;
        }else{
            ch2 -= 0x41;
            ch2 += 10;
        }
        content2.push((ch1 << 4) + ch2);
    }
    return Ok(content2);
}

// 用于解析red数组的方法
fn parse_arr<'a>(arr_data: &'a str) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> {
    let err_str = "can not get arr type";
    if !arr_data.starts_with("12331549-6D26-68A5-E192-5EBE9A6EB998") {
        return Err(err_str.into());
    }
    let tp = arr_data.get(36..37).ok_or(err_str)?;
    if tp != "A" {
        return Err(err_str.into());
    }
    let mut ret_arr:Vec<&str> = vec![];
    let mut arr = arr_data.get(37..).ok_or(err_str)?;
    loop {
        let spos_opt = arr.find(",");
        if let None = spos_opt {
            break;
        }
        let spos_num = spos_opt.ok_or(err_str)?;
        let num_opt = arr.get(0..spos_num);
        let num_str = num_opt.ok_or(err_str)?;
        let num = num_str.parse::<usize>()?;
        let str_val = arr.get(spos_num + 1..spos_num + 1 + num).ok_or(err_str)?;
        ret_arr.push(str_val);
        arr = arr.get(spos_num + 1 + num..).ok_or(err_str)?;
    }
    return Ok(ret_arr);
}

/// 这个函数是必须写的，否则会认为不是红色问答的插件
/// 这个函数会下发一个身份识别码，你需要保存它，之后调用某些api会用到
/// 这个函数需要返回支持的红色问答API版本
#[no_mangle]
pub extern "system" fn redreply_api_version(ac:c_int) -> c_int {
    let mut lk = G_AC.lock().unwrap();
    (*lk) = ac;
    return 1;
}

/// 返回你自己的插件版本
/// 这个函数不是必须的
#[no_mangle]
pub extern "system" fn redreply_lib_version() -> c_int {
    return 1;
}

/// 用于向红色问答注册一个或多个命令
/// 需要返回一个red数组
/// 这个函数不是必须的
#[no_mangle]
pub extern "system" fn redreply_regist_cmd(ctx:*const c_void,callback: extern "system" fn (*const c_void,*const c_char)) {
    let cmds = ["__TXSILK"]; // 需要注册的命令
    let mut to_ret = String::from_str("12331549-6D26-68A5-E192-5EBE9A6EB998A").unwrap();
    for cmd in cmds {
        to_ret.push_str(&format!("{},{}",cmd.len(),cmd));
    }
    let to_ret_cstr = CString::new(to_ret).unwrap();
    // ctx 需要原样返回
    callback(ctx,to_ret_cstr.as_ptr());
}


/// 这个函数用于处理来自插件的命令调用
/// ctx需要原样返回
/// cmd_name_raw 是命令的名字
/// args_arr_raw 是命令的参数，是一个red数组，你需要解析它
/// 回调函数第一个参数是ctx，原样返回
/// 回调函数第二个参数是命令的返回，需要处理成redlang数据格式
/// 回到函数第三个参数表示命令执行是否成功，0表示成功，其它表示失败
/// 失败时，可以使用回调函数第二个参数说明失败原因
#[no_mangle]
pub extern "system" fn redreply_callcmd(ctx:*const c_void,cmd_name_raw:*const c_char,args_arr_raw:*const c_char,callback: extern "system" fn (*const c_void,*const c_char,c_int)) {
    // 如果不是支持的命令，直接返回
    let cmd_name = unsafe { CStr::from_ptr(cmd_name_raw) };
    if cmd_name.to_str().unwrap_or_default() != "__TXSILK" {
        let not_found = CString::new("cmd not found").unwrap();
        callback(ctx,not_found.as_ptr(),404);
    }
    // 解析参数数组
    let args_arr = unsafe { CStr::from_ptr(args_arr_raw) };
    let args_rst = parse_arr(args_arr.to_str().unwrap());
    if args_rst.is_err() {
        let err_str = CString::new(format!("{}",args_rst.err().unwrap().to_string())).unwrap();
        callback(ctx,err_str.as_ptr(),-2);
        return;
    }
    let args_arr = args_rst.unwrap();
    if args_arr.len() < 1 {
        let not_found = CString::new("params arr not enough len").unwrap();
        callback(ctx,not_found.as_ptr(),-3);
        return;
    }
    // 处理参数
    let from_bin_str = args_arr[0]; // 要转silk的语音文本
    let from_bin_rst = parse_bin(from_bin_str);
    if from_bin_rst.is_err() {
        let can_not_get_bin = CString::new("can't get bin").unwrap();
        callback(ctx,can_not_get_bin.as_ptr(),-4);
        return;
    }
    let from_bin = from_bin_rst.unwrap();

    // let media_type = get_media_type(&from_bin);

    let rst_silk = all_to_silk::all_to_silk(&from_bin);
    if rst_silk.is_err() {
        let conv_err = CString::new(format!("conv_err:{:?}",rst_silk.err())).unwrap();
        callback(ctx,conv_err.as_ptr(),-5);
        return;
    }

    let silk = rst_silk.unwrap();

    // 这里在构造redlang字节集类型
    let mut to_ret = String::from_str("12331549-6D26-68A5-E192-5EBE9A6EB998B").unwrap();
    for ch in silk {
        to_ret.push_str(&format!("{:02X}",ch));
    }
    // 命令返回
    let to_ret_c = CString::new(to_ret).unwrap();
    callback(ctx,to_ret_c.as_ptr(),0);
    return;
    
}
