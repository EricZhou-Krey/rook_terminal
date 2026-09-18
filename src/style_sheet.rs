pub const ICON: &str = "
                             ...                                      
                        .-+#@#-                                       
              :#%%%##+*@@@@@=                   :....  .              
              +@@@@#. %@@@%.                   .=--..:..... .         
              +@@=    %@@*                     .=:.---:....           
              :=      %@*                       :-===---.             
            .#@.      @#.                      :+++===-.              
           -@@@.      +-                      =***+++==-:::.          
          -@@@@.                          :+*%###***++=-:             
         =@@@@@.     =:                 =%@@@%%%###*=-=+=:            
        -@@@@@@.     #+               =@@@@@@@@%%###*+:  :-.          
       .@@@@@@@.     #+             -@@@@@@@@@@@%%#++**+:             
       +@@@@@@@.     #+       :=:. @@@@@@@@@@@@@@@%=   -*=.           
      .@@@@@@@@.     #+  -@@@@@@@@@@@@@@@@@@@@@@@@@%%=   .=.          
      =@@@@@@@@.     =-  ..-+@@@@@@@@@@@@@@@@@@@@@@@@%+.              
      *@@@@@@@@.               @@@@@@@@@@@@@@@@@@@@@=-##:             
     .@@@@@@@@@.      -:  :+#@@@@@@@@@@@@@@@@@@@@@@@@+.:+:            
     .@@@@@@@@@.      =+#@@@@@@@@@@@@@@@@@@@@@@@@@#=+%*. -:           
      %@@@@@@@@.     -@@@@@@@@@@@@@@@@@@@@@@@@@@@@*=-:*=              
      +@@@@@@@@.   .#@@@@@@@@@@@@@@@@@@@@@@@:#=*@#%=-=--.             
      -@@@@@@@@.  -@@@@@@@@@@@@@@@@@@@@@@@@@+@@@*%+*:+@+              
      .@@@@@@@%:=@@@@@@@@@@@@@@@@@@@@@@@@@@@#@@@%+#-%@@+              
       =@@@@**@@@@@@@@@@@@@+@@@@-=@@@@@@@@@% ..+@%=*##@+              
        +#=@@-..%@@@@@@@@@+=@-@-@@:@@@@@@@@+   =@@.:-@@+              
         :+@@%*@@@#@@@@@#: %:    .:@@@@@@@@:   =@@@@@@@*              
         :@@+@@-:.*@*.@@+         +@@@@@@@@=   =@@@@@@@*      =%.     
          .+%**   #@: @%.        =@@@@@@@@@@ . =@@@@@@@#=%@@@@@:      
         .++=@@.  ##           :-@@@@@@@@@@@%*@@.      %@@@@@%:       
        .*=  =*.  ##          #-@@@@@@@@@@@%#*@@.      %@@@@+         
       :%-    :+: ##        :@#@@@@@@@@@@@@%@@@@.      %@@%.          
      :@:      #@%#*       =@@+@@@@@@@@@@@@%#@@@.      *#.            
     .@.       .%@@-..    %@@@@*@@+@:@@+-:*@@@@@.    =@*              
     *.          *#:@@@*-@@@@@@=@-@@=@@=@@@@@@@@..+@@@@*              
    :=             .--:.  .=@@#@-@@@*@@#@@@@@@+:-======.              
    +                         .#..::.@=...                            
                              *.     #.                               
                              +      #.                               
                                     +=                               
";

pub const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);
pub const PROMPT_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 250, 120);
pub const SELECTION_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(100, 100, 100, 100);
pub const BACKGROUND_CORNER_RADIUS: f32 = 0.0;
pub const BACKGROUND_COLOR: egui::Color32 = egui::Color32::from_rgb(27, 27, 27);
pub const TEXT_STYLE: egui::TextStyle = egui::TextStyle::Monospace;

pub const ANSI_BLACK: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
pub const ANSI_RED: egui::Color32 = egui::Color32::from_rgb(204, 36, 29);
pub const ANSI_GREEN: egui::Color32 = egui::Color32::from_rgb(152, 151, 26);
pub const ANSI_YELLOW: egui::Color32 = egui::Color32::from_rgb(215, 153, 33);
pub const ANSI_BLUE: egui::Color32 = egui::Color32::from_rgb(69, 133, 136);
pub const ANSI_MAGENTA: egui::Color32 = egui::Color32::from_rgb(177, 98, 134);
pub const ANSI_CYAN: egui::Color32 = egui::Color32::from_rgb(104, 157, 106);
pub const ANSI_WHITE: egui::Color32 = egui::Color32::from_rgb(235, 219, 178);

pub const ANSI_BRIGHT_BLACK: egui::Color32 = egui::Color32::from_rgb(146, 131, 116);
pub const ANSI_BRIGHT_RED: egui::Color32 = egui::Color32::from_rgb(251, 73, 52);
pub const ANSI_BRIGHT_GREEN: egui::Color32 = egui::Color32::from_rgb(184, 187, 38);
pub const ANSI_BRIGHT_YELLOW: egui::Color32 = egui::Color32::from_rgb(250, 189, 47);
pub const ANSI_BRIGHT_BLUE: egui::Color32 = egui::Color32::from_rgb(131, 165, 152);
pub const ANSI_BRIGHT_MAGENTA: egui::Color32 = egui::Color32::from_rgb(211, 134, 155);
pub const ANSI_BRIGHT_CYAN: egui::Color32 = egui::Color32::from_rgb(142, 192, 124);
pub const ANSI_BRIGHT_WHITE: egui::Color32 = egui::Color32::from_rgb(253, 244, 193);

pub fn apply_ansi_code(
    code_string: &str,
    text_format: &mut egui::text::TextFormat,
    default_text_color: egui::Color32,
    default_background_color: egui::Color32,
) {
    match code_string {
        "0" => {
            text_format.color = default_text_color;
            text_format.background = default_background_color;
            text_format.italics = false;
            text_format.underline = egui::Stroke::NONE;
            text_format.strikethrough = egui::Stroke::NONE;
        }
        "1" => {}
        "3" => {
            text_format.italics = true;
        }
        "4" => {
            text_format.underline = egui::Stroke::new(1.0, text_format.color);
        }
        "9" => {
            text_format.strikethrough = egui::Stroke::new(1.0, text_format.color);
        }
        "22" => {}
        "23" => {
            text_format.italics = false;
        }
        "24" => {
            text_format.underline = egui::Stroke::NONE;
        }
        "29" => {
            text_format.strikethrough = egui::Stroke::NONE;
        }
        "30" => {
            text_format.color = ANSI_BLACK;
        }
        "31" => {
            text_format.color = ANSI_RED;
        }
        "32" => {
            text_format.color = ANSI_GREEN;
        }
        "33" => {
            text_format.color = ANSI_YELLOW;
        }
        "34" => {
            text_format.color = ANSI_BLUE;
        }
        "35" => {
            text_format.color = ANSI_MAGENTA;
        }
        "36" => {
            text_format.color = ANSI_CYAN;
        }
        "37" => {
            text_format.color = ANSI_WHITE;
        }
        "39" => {
            text_format.color = default_text_color;
        }
        "40" => {
            text_format.background = ANSI_BLACK;
        }
        "41" => {
            text_format.background = ANSI_RED;
        }
        "42" => {
            text_format.background = ANSI_GREEN;
        }
        "43" => {
            text_format.background = ANSI_YELLOW;
        }
        "44" => {
            text_format.background = ANSI_BLUE;
        }
        "45" => {
            text_format.background = ANSI_MAGENTA;
        }
        "46" => {
            text_format.background = ANSI_CYAN;
        }
        "47" => {
            text_format.background = ANSI_WHITE;
        }
        "49" => {
            text_format.background = default_background_color;
        }
        "90" => {
            text_format.color = ANSI_BRIGHT_BLACK;
        }
        "91" => {
            text_format.color = ANSI_BRIGHT_RED;
        }
        "92" => {
            text_format.color = ANSI_BRIGHT_GREEN;
        }
        "93" => {
            text_format.color = ANSI_BRIGHT_YELLOW;
        }
        "94" => {
            text_format.color = ANSI_BRIGHT_BLUE;
        }
        "95" => {
            text_format.color = ANSI_BRIGHT_MAGENTA;
        }
        "96" => {
            text_format.color = ANSI_BRIGHT_CYAN;
        }
        "97" => {
            text_format.color = ANSI_BRIGHT_WHITE;
        }
        "100" => {
            text_format.background = ANSI_BRIGHT_BLACK;
        }
        "101" => {
            text_format.background = ANSI_BRIGHT_RED;
        }
        "102" => {
            text_format.background = ANSI_BRIGHT_GREEN;
        }
        "103" => {
            text_format.background = ANSI_BRIGHT_YELLOW;
        }
        "104" => {
            text_format.background = ANSI_BRIGHT_BLUE;
        }
        "105" => {
            text_format.background = ANSI_BRIGHT_MAGENTA;
        }
        "106" => {
            text_format.background = ANSI_BRIGHT_CYAN;
        }
        "107" => {
            text_format.background = ANSI_BRIGHT_WHITE;
        }
        _ => {}
    }
}
