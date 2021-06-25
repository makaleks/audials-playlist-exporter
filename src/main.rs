use std::path::PathBuf;

use iced::{Application, Button, Row, Text, button, button::Style};

//const MyFont: iced::Font = iced::Font::External {
//    name: "OpenSans-Regular",
//    bytes: include_bytes!("../OpenSans-Regular.ttf"),
//};

const MY_BASE_FONT_SIZE: u16 = 32;

fn main() -> iced::Result {
    let mut iced_settings = iced::Settings::default();
    iced_settings.default_font = Some(include_bytes!("../OpenSans-Regular.ttf"));
    iced_settings.antialiasing = true;
    iced_settings.default_text_size = MY_BASE_FONT_SIZE;
    MainApp::run(iced_settings)
}

struct BaseMenuData {
    audilas_path: PathBuf,
    library_path: PathBuf,
    validation_required: bool,
    is_opened: bool,
}
struct BaseMenuIcedStates {
    btn_update_audilas_path:    iced::button::State,
    btn_update_library_path:    iced::button::State,
    btn_update_output_path:     iced::button::State,
    btn_validate_or_open_close: iced::button::State,
}

struct BaseMenu {
    data:   BaseMenuData,
    states: BaseMenuIcedStates,
}

struct MyJsonEntryValue {
    parsed_value: Vec<serde_json::Value>,
    file_path:    PathBuf,
}
impl ToString for MyJsonEntryValue {
    fn to_string(&self) -> String {
        self.file_path.to_string_lossy().into_owned()
    }
}
struct PathBufWrapper(PathBuf);
impl ToString for PathBufWrapper {
    fn to_string(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}
enum MyFileEntry<T: ToString> {
    NotInited,
    Valid(T),
    InvalidWithError(MyError),
}
impl<T: ToString> MyFileEntry<T> {
    fn new () -> MyFileEntry<T> {
        MyFileEntry::NotInited
    }
    fn is_valid(&self) -> bool {
        match self {
            MyFileEntry::Valid(_) => true,
            _                     => false,
        }
    }
    fn to_iced_short_text(&self) -> iced::Text {
        match self {
            MyFileEntry::NotInited => gen_text(""),
            MyFileEntry::Valid(value) => gen_text(value.to_string().as_str()),
            MyFileEntry::InvalidWithError(err) => gen_text(err.short_error).color([1.0, 0.0, 0.0])
        }
    }
    fn to_iced_full_text(&self) -> iced::Text {
        match self {
            MyFileEntry::NotInited => gen_text(""),
            MyFileEntry::Valid(value) => gen_text(value.to_string().as_str()),
            MyFileEntry::InvalidWithError(err) => gen_text(err.full_error.as_str()).color([1.0, 0.0, 0.0])
        }
    }
}

fn get_error_str_for_json_entry (value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "обнаружен null",
        serde_json::Value::String(_) => "обнаружена строка",
        serde_json::Value::Number(_) => "обнаружено число",
        serde_json::Value::Bool(_) => "обнаружено булево значение",
        serde_json::Value::Object(_) => "обнаружен объект (таблица ключ-значение)",
        serde_json::Value::Array(_) => "обнаружен массив",
    }
}

fn file_entry_from_json_file (filepath: PathBuf) -> MyFileEntry<MyJsonEntryValue> {
    let content = match std::fs::read_to_string(&filepath) {
        Ok(strings) => strings,
        Err(_) => {
            return MyFileEntry::InvalidWithError(MyError{
                full_error: format!("# Ошибка: не удалось открыть файл '{}'", filepath.to_string_lossy()),
                short_error: "Не удалось открыть файл",
            });
        }
    };
    let mut lines = content.lines();
    if let None = lines.next() {
        return MyFileEntry::InvalidWithError(MyError{
            full_error: format!("# Ошибка: ожидалось минимум 2 строки в файле '{}'", filepath.to_string_lossy()),
            short_error: "В файле не двух строк",
        });
    }
    else {
        // ok
        match lines.next() {
            None => {
                return MyFileEntry::InvalidWithError(MyError{
                    full_error: format!("# Ошибка: ожидалось минимум 2 строки в файле '{}'", filepath.to_string_lossy()),
                    short_error: "В файле не двух строк",
                });
            }
            Some(json_candidate) => {
                // ok
                let value = match serde_json::from_str(json_candidate) {
                    Ok(json) => json,
                    Err(error) => {
                        return MyFileEntry::InvalidWithError(MyError{
                            full_error: format!(
                                "# Ошибка: не удалось разобрать файл '{}', в позиции [{}:{}]: {}",
                                filepath.to_string_lossy(), 1+error.line(), error.column(),
                                match error.classify() {
                                    serde_json::error::Category::Io => "ошибка при чтении",
                                    serde_json::error::Category::Syntax => "ошибка синтаксиса, формат отличен от JSON",
                                    serde_json::error::Category::Data => "данные этого JSON не согласованы",
                                    serde_json::error::Category::Eof => "JSON закончился слишком рано",
                                }
                            ),
                            short_error: "Не удалось разобрать файл",
                        });
                    }
                };
                if let serde_json::Value::Array(value_with_expected_format) = value {
                    // ok
                    return MyFileEntry::Valid(MyJsonEntryValue{
                        parsed_value: value_with_expected_format,
                        file_path:    filepath,
                    });
                }
                else {
                    return MyFileEntry::InvalidWithError(MyError{
                        full_error: format!(
                            "# Ошибка: неожиданный формат JSON в файле '{}', ожидался массив, {}",
                            filepath.to_string_lossy(),
                            get_error_str_for_json_entry(&value)
                        ),
                        short_error: "Не удалось разобрать файл",
                    });
                }
            }
        }
    }
}

fn file_entry_from_audio_sqlite_file (filepath: PathBuf) -> MyFileEntry<PathBufWrapper> {
    // https://stackoverflow.com/a/21146372
    let connection = match rusqlite::Connection::open_with_flags(&filepath, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => {
            return MyFileEntry::InvalidWithError(MyError{
                full_error: format!("# Ошибка: не удалось открыть базы данных '{}'", filepath.to_string_lossy()),
                short_error: "Не удалось открыть файл",
            });
        }
    };
    if let Err(_) = connection.execute_batch("pragma schema_version;") {
        return MyFileEntry::InvalidWithError(MyError{
            full_error: format!("# Ошибка: нельзя распознать как базу данных SQLite файл '{}'", filepath.to_string_lossy()),
            short_error: "Не база данных",
        });
    }
    MyFileEntry::Valid(PathBufWrapper(filepath))
}

fn pathbuf_from_pattern (base_dir: &PathBuf, pattern: &str) -> PathBuf {
    // expected pattern: /some/dir/*filename-end
    let pattern_as_path = std::path::Path::new(pattern);
    let pattern_base_dir = pattern_as_path.parent().unwrap();
    let file_ending_by_pattern = pattern_as_path.file_name().unwrap().to_str().unwrap().strip_prefix("*").unwrap();

    let mut base_dir = base_dir.clone();
    base_dir.push(pattern_base_dir);
    if !base_dir.exists() {
        return PathBuf::new();
    }
    if let Ok(read_dir) = base_dir.read_dir() {
        for entry in read_dir {
            match entry {
                Ok(entry) => if let Some(utf8_filename) = entry.file_name().to_str() {
                    if utf8_filename.ends_with(file_ending_by_pattern) {
                        base_dir.push(entry.file_name());
                        return base_dir;
                    }
                },
                Err(_) => break,
            }
        }
    }
    PathBuf::new()
}

struct DataFilesMenuData {
    is_opened:        bool,
    playlists:        MyFileEntry<MyJsonEntryValue>,
    playlist_entries: MyFileEntry<MyJsonEntryValue>,
    audio_database:   MyFileEntry<PathBufWrapper>,

    is_valid_prev:    bool,
}
impl DataFilesMenuData {
    fn is_valid (&self) -> bool {
        self.audio_database.is_valid() && self.playlists.is_valid() && self.playlist_entries.is_valid()
    }
    fn is_initial (&self) -> bool {
        match (&self.playlists, &self.playlist_entries, &self.audio_database) {
            (MyFileEntry::NotInited, MyFileEntry::NotInited, MyFileEntry::NotInited) => true,
            _ => false,
        }
    }
    fn init_auto (&mut self, base_menu_data: &BaseMenuData) {
        self.playlists = file_entry_from_json_file(pathbuf_from_pattern(&base_menu_data.audilas_path, "LocalAppDataFolder/RapidSolution/Audials_2015/AudialsSync/*_playlists.txt"));
        self.playlist_entries = file_entry_from_json_file(pathbuf_from_pattern(&base_menu_data.audilas_path, "LocalAppDataFolder/RapidSolution/Audials_2015/AudialsSync/*_playlistentries.txt"));
        self.audio_database = file_entry_from_audio_sqlite_file({
            let mut audio_path = base_menu_data.audilas_path.clone();
            audio_path.push("LocalAppDataFolder/RapidSolution/Audials_2015/MusicOrganizer/modb");
            audio_path
        });
    }
}
struct DataFilesMenuIcedStates {
    btn_update_playlists_path:        iced::button::State,
    btn_update_playlist_entries_path: iced::button::State,
    btn_update_audio_database_path:   iced::button::State,
    btn_validate_or_open_close:       iced::button::State,
}
struct DataFilesMenu {
    data:   DataFilesMenuData,
    states: DataFilesMenuIcedStates,
}

impl DataFilesMenu {
    fn new () -> DataFilesMenu {
        DataFilesMenu {
            data: DataFilesMenuData {
                is_opened:        false,
                playlists:        MyFileEntry::new(),
                playlist_entries: MyFileEntry::new(),
                audio_database:   MyFileEntry::new(),
                is_valid_prev:    false,
            },
            states: DataFilesMenuIcedStates {
                btn_update_playlists_path:        iced::button::State::new(),
                btn_update_playlist_entries_path: iced::button::State::new(),
                btn_update_audio_database_path:   iced::button::State::new(),
                btn_validate_or_open_close:       iced::button::State::new(),
            }
        }
    }
    fn reset_me (&mut self) {
        *self = DataFilesMenu::new();
    }
    fn update_if_acceptable (&mut self, message: &MyMessage) {
        if [
            MyMessage::SetPlaylistsPath, MyMessage::SetPlaylistEntriesPath, MyMessage::SetAudioDatabasePath
        ].contains(&message) {
            if let Some(new_file_path) = rfd::FileDialog::new().pick_file() {
                match message {
                    MyMessage::SetPlaylistsPath => {
                        self.data.playlists = file_entry_from_json_file(new_file_path);
                    },
                    MyMessage::SetPlaylistEntriesPath => {
                        self.data.playlist_entries = file_entry_from_json_file(new_file_path);
                    },
                    MyMessage::SetAudioDatabasePath => {
                        self.data.audio_database = file_entry_from_audio_sqlite_file(new_file_path);
                    },
                    _ => unreachable!(),
                }
            }
        }
        else if [
            MyMessage::OpenDataFilesMenu, MyMessage::CloseDataFilesMenu
        ].contains(&message) {
            match message {
                MyMessage::OpenDataFilesMenu  => self.data.is_opened = true,
                MyMessage::CloseDataFilesMenu => self.data.is_opened = false,
                _ => unreachable!(),
            }
        }
    }
    fn view<'a> (data: &'a DataFilesMenuData, states: &'a mut DataFilesMenuIcedStates) -> iced::Element<'a, MyMessage> {
        let mut column = iced::widget::Column::new();
        if data.is_opened {
            column = column
                .push(gen_text("Путь до файла с плейлистами"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_playlists_path, gen_text("Обновить")).on_press(MyMessage::SetPlaylistsPath)
                        )
                        .push(data.playlists.to_iced_short_text())
                )
                .push(gen_text("Путь до файла со списком вхождений в плейлист"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_playlist_entries_path, gen_text("Обновить")).on_press(MyMessage::SetPlaylistEntriesPath)
                        )
                        .push(data.playlist_entries.to_iced_short_text())
                )
                .push(gen_text("Путь до базы данных с музыкой"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_audio_database_path, gen_text("Обновить")).on_press(MyMessage::SetAudioDatabasePath)
                        )
                        .push(data.audio_database.to_iced_short_text())
                );
        }
        column.push(iced::Row::new().push(iced::Button::new(&mut states.btn_validate_or_open_close, gen_text(
                if data.is_opened {
                    "Свернуть"
                }
                else {
                    "Открыть настройки местоположения файлов Audials снова"
                }
            )).on_press(
                if data.is_opened {
                    MyMessage::CloseDataFilesMenu
                }
                else {
                    MyMessage::OpenDataFilesMenu
                }
            )).push(iced::Rule::horizontal(MY_BASE_FONT_SIZE)))
            .into()
    }
}

struct MyError {
    short_error: &'static str,
    full_error:  String,
}
// Результат и строка ошибки
type MyResult<T> = Result<T, String>;

impl BaseMenu {
    fn new () -> BaseMenu {
        BaseMenu {
            data: BaseMenuData {
                audilas_path: "/run/media/makaleks/FreeAgent GoFlex Drive/Audials_12_0_60600_0_portable".into(),
                library_path: "/run/media/makaleks/FreeAgent GoFlex Drive/Audials Music".into(),
                validation_required: true,
                is_opened: true,
            },
            states: BaseMenuIcedStates {
                btn_update_audilas_path:    iced::button::State::new(),
                btn_update_library_path:    iced::button::State::new(),
                btn_update_output_path:     iced::button::State::new(),
                btn_validate_or_open_close: iced::button::State::new(),
            }
        }
    }
    fn view<'a> (data: &'a BaseMenuData, states: &'a mut BaseMenuIcedStates) -> iced::Element<'a, MyMessage> {
        let mut column = iced::widget::Column::new();
        if data.is_opened {
            column = column
                .push(gen_text("Путь до Audials - для автоматического поиска плейлистов"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_audilas_path, gen_text("Обновить")).on_press(MyMessage::SetAudilasPath)
                        )
                        .push(
                            gen_text(data.audilas_path.as_path().to_string_lossy().into_owned().as_str())
                ))
                .push(gen_text("Путь до библиотеки с музыкой (папка должна иметь хотя бы собственное имя то же, что оставлено в базе Audials)"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_library_path, gen_text("Обновить")).on_press(MyMessage::SetLibraryPath)
                        )
                        .push(
                            gen_text(data.library_path.as_path().to_string_lossy().into_owned().as_str())
                ));
        }
        column.push(iced::Row::new().push(iced::Button::new(&mut states.btn_validate_or_open_close, gen_text(
                if data.validation_required {
                    "Проверить"
                }
                else if data.is_opened {
                    "Свернуть"
                }
                else {
                    "Открыть базовые настройки снова"
                }
            )).on_press(
                if data.validation_required {
                    MyMessage::ValidateBaseMenu
                }
                else if data.is_opened {
                    MyMessage::CloseBaseMenu
                }
                else {
                    MyMessage::OpenBaseMenu
                }
            )).push(iced::Rule::horizontal(MY_BASE_FONT_SIZE)))
            .into()
    }
    fn update_if_acceptable (&mut self, message: &MyMessage) {
        if [
            MyMessage::SetAudilasPath, MyMessage::SetLibraryPath, MyMessage::SetOutputPath
        ].contains(&message) {
            if let Some(new_dir_path) = rfd::FileDialog::new().pick_folder() {
                match message {
                    MyMessage::SetAudilasPath if self.data.audilas_path != new_dir_path => {
                        self.data.audilas_path = new_dir_path;
                        self.data.validation_required = true;
                    },
                    MyMessage::SetLibraryPath if self.data.library_path != new_dir_path => {
                        self.data.library_path = new_dir_path;
                        self.data.validation_required = true;
                    },
                    _ => unreachable!(),
                }
            }
        }
        else if [
            MyMessage::ValidateBaseMenu, MyMessage::OpenBaseMenu, MyMessage::CloseBaseMenu
        ].contains(&message) {
            match message {
                MyMessage::OpenBaseMenu  => self.data.is_opened = true,
                MyMessage::CloseBaseMenu => self.data.is_opened = false,
                MyMessage::ValidateBaseMenu => {
                    self.data.validation_required = false;
                    self.data.is_opened           = false;
                },
                _ => unreachable!(),
            }
        }
    }
}

//fn slice_u8_to_slice_u16_from_begin_lossy<'a> (bytes: &[u8]) -> &'a [u16] {
//    // waiting for `as_chunks()` stabilization
//    let dest_bytes_len = if 0 == bytes.len() % 2 {bytes.len()} else {bytes.len() - 1};
//    unsafe {
//        std::slice::from_raw_parts(bytes.as_ptr() as *const u16, dest_bytes_len)
//    }
//}

#[derive(Debug)]
struct AudioEntry {
    title: String,
    artist: String,
    path: String,
}
struct PlaylistEntry {
    name: String,
    id: String,
}
impl PlaylistEntry {
    fn from_json_object (obj: &serde_json::Map<String, serde_json::Value>, playlist_error_log: &mut Vec<String>) -> Option<PlaylistEntry> {
        // Это какой-то бред: audials по полу payload указывает не на объект, а
        // на Строковое Представление объекта, то есть нужно лишний раз гонять
        // Строку->в объект->в поле объекта->в строковое поле объекта
        match (obj.get("payload"), obj.get("id")) {
            (Some(payload_value), Some(id_value)) => match (serde_json::from_value::<String>(payload_value.clone()), serde_json::from_value(id_value.clone())) {
                (Ok(payload_string), Ok(id_string)) => match serde_json::from_str::<serde_json::Value>(payload_string.as_str()) {
                    Ok(payload_obj) => match payload_obj.get("Name") {
                        Some(name_value) => {
                            if let Ok(name_string) = serde_json::from_value::<String>(name_value.clone()) {
                                //println!("name: {}, [{}]", name_string, name_string.bytes().map(|b|format!("{:02x} ", b)).collect::<String>());
                                return Some(PlaylistEntry{
                                    name: name_string,
                                    id: id_string
                                });
                            }
                            else {
                                playlist_error_log.push(format!("# Ошибка: для поля Name ожидалась строка, {}", get_error_str_for_json_entry(&name_value)));
                                return None;
                            }
                        }
                        _ => {
                            playlist_error_log.push(format!("# Ошибка: не удалось найти в объекте payload '{}' поле Name", payload_obj));
                            return None;
                        }
                    }
                    _ => {
                        playlist_error_log.push(format!("# Ошибка: не удалось распознать строку в поле payload '{}' как объект JSON", payload_string));
                        return None;
                    }
                }
                _ => {
                    playlist_error_log.push(
                        format!("# Ошибка: неожиданный формат JSON в имени или идентификаторе плейлиста, ожидались строки, для payload '{}', для идентификатора '{}'", get_error_str_for_json_entry(&payload_value), get_error_str_for_json_entry(&id_value))
                    );
                    return None;
                }
            },
            _ => {
                playlist_error_log.push(
                    "# Ошибка: не удалось найти в объекте плейлиста поля payload или id".to_string()
                );
                return None;
            },
        }
    }
}
struct SelectionMenuData {
    is_opened: bool,
    is_validation_required: bool,

    playlists: Vec<PlaylistEntry>,
    selected_playlist: Option<String>,
    playlists_error_log: Vec<String>, // ошибки в формировании списка плейлистов
    playlist_test_error_log: Vec<String>, // ошибки в формировании содержимого плейлиста и существования файлов

    audio_in_playlist: Vec<AudioEntry>,
    //audio_count: u32,

    output_path: PathBuf,
    is_exported: bool,
}
impl SelectionMenuData {
    fn init (&mut self, data_files_menu_data: &DataFilesMenuData) {
        self.playlists_error_log.clear();
        for arr_entry in match &data_files_menu_data.playlists {
            MyFileEntry::Valid(structure) => &structure.parsed_value,
            _ => unreachable!()
        } {
            match arr_entry {
                serde_json::Value::Object(obj) => {
                    PlaylistEntry::from_json_object(obj, &mut self.playlists_error_log).map(|entry| self.playlists.push(entry));
                },
                _ => {
                    self.playlists_error_log.push(
                        format!("# Ошибка: неожиданный формат JSON в массиве плейлистов, ожидался объект (таблица ключ-значение), {}", get_error_str_for_json_entry(arr_entry))
                    )
                }
            }
        }
    }
}
struct SelectionMenuIcedStates {
    pck_playlist_select: iced::widget::pick_list::State<String>,
    btn_update_output_path: iced::widget::button::State,
    scrl_audios: iced::widget::scrollable::State,
    btn_test: iced::widget::button::State,
    btn_open_close: iced::widget::button::State,
}
struct SelectionMenu {
    data:   SelectionMenuData,
    states: SelectionMenuIcedStates,
}
impl SelectionMenu {
    fn new () -> SelectionMenu {
        SelectionMenu {
            data: SelectionMenuData {
                is_validation_required: true,
                is_opened: true,
                playlists: Vec::new(),
                selected_playlist: None,
                playlists_error_log: Vec::new(),
                playlist_test_error_log: Vec::new(),
                output_path: dirs_next::home_dir().unwrap(),
                audio_in_playlist: Vec::new(),
                //audio_count: 0,
                is_exported: false,
            },
            states: SelectionMenuIcedStates {
                pck_playlist_select: iced::widget::pick_list::State::default(),
                btn_update_output_path: iced::widget::button::State::new(),
                scrl_audios: iced::widget::scrollable::State::new(),
                btn_test: iced::widget::button::State::new(),
                btn_open_close: iced::widget::button::State::new(),
            },
        }
    }
    fn reset_me (&mut self) {
        *self = SelectionMenu::new();
    }
    fn view<'a> (data: &'a SelectionMenuData, states: &'a mut SelectionMenuIcedStates) -> iced::Element<'a, MyMessage> {
        let mut column = iced::widget::Column::new();
        if data.is_opened {
            let mut audio_scroll = iced::widget::Scrollable::new(&mut states.scrl_audios);
            if !data.is_validation_required {
                audio_scroll = audio_scroll.max_height((6*MY_BASE_FONT_SIZE).into()).push(gen_text(
                    format!("Будет экспортирован{} {} {}",
                        if 1 == data.audio_in_playlist.len() %10 {""} else {"о"},
                        data.audio_in_playlist.len(),
                        match data.audio_in_playlist.len() % 10 {
                            1 => "файл",
                            2..=4 => "файла",
                            _ => "файлов",
                        }
                    ).as_str()
                ));
                if 50 < data.audio_in_playlist.len() {
                    audio_scroll = audio_scroll.push(gen_text("Первые 50 файлов:"));
                }
                for (i, audio) in data.audio_in_playlist.iter().enumerate() {
                    if 50 == i {
                        break;
                    }
                    audio_scroll = audio_scroll.push(iced::widget::Row::new()
                        .push(gen_text(format!("{} # ", audio.title).as_str()))
                        .push(gen_text(format!("{} # ", audio.artist).as_str()))
                        .push(gen_text(std::path::Path::new(&audio.path).file_name().unwrap().to_str().unwrap()))
                    );
                }
            }

            let mut menu_column = iced::widget::Column::new()
                .push(gen_text("Путь до директории с результатом"))
                .push(iced::Row::new()
                        .push(
                            iced::Button::new(&mut states.btn_update_output_path, gen_text("Обновить")).on_press(MyMessage::SetOutputPath)
                        )
                        .push(
                            gen_text(data.output_path.as_path().to_string_lossy().into_owned().as_str())
                ))
                .push(gen_text("Плейлист:"))
                .push(iced::widget::PickList::new(&mut states.pck_playlist_select, data.playlists.iter().map(|entry| entry.name.clone()).collect::<Vec<String>>(), data.selected_playlist.clone(), MyMessage::SelectPlaylist));
            if let Some(_) = data.selected_playlist {
                menu_column = if data.is_validation_required {
                    menu_column.push(iced::Button::new(&mut states.btn_test, gen_text("Проверить")).on_press(MyMessage::TestPlaylist))
                }
                else {
                    menu_column.push(iced::Button::new(&mut states.btn_test, gen_text("Экспортировать")).on_press(MyMessage::Export))
                }
            }

            let mut row = iced::widget::Row::new();
            row = row
                .push(menu_column)
                .push(audio_scroll);
            column = column.push(row);
        }
        column.push(iced::Row::new().push(iced::Button::new(&mut states.btn_open_close, gen_text(
                if data.is_opened {
                    "Сверуть"
                }
                else {
                    "Открыть выбор плейлистов снова"
                }
            )).on_press(
                if data.is_opened {
                    MyMessage::CloseSelectionMenu
                }
                else {
                    MyMessage::OpenSelectionMenu
                }
            ))).push(iced::Rule::horizontal(MY_BASE_FONT_SIZE))
            .into()
    }
    fn update_if_acceptable (&mut self, message: &MyMessage, base_menu_data: &BaseMenuData, data_files_menu_data: &DataFilesMenuData) {
        match message {
            MyMessage::SetOutputPath =>
                if let Some(new_dir_path) = rfd::FileDialog::new().pick_folder() {
                    if self.data.output_path != new_dir_path {
                        self.data.output_path = new_dir_path;
                        self.data.is_validation_required = true;
                        self.data.is_exported = false;
                    }
                },
            MyMessage::SelectPlaylist(playlist_name) => {
                self.data.playlist_test_error_log.clear();
                self.data.selected_playlist = Some(playlist_name.clone());
                self.data.is_validation_required = true;
                self.data.is_exported = false;
                //println!("selected {}", pl);
            },
            MyMessage::TestPlaylist => {
                self.data.playlist_test_error_log.clear();
                self.data.is_exported = false;
                let audio_ids = get_entries_ids_for_playlist(
                    &self.data.playlists.iter().find(|entry| &entry.name == self.data.selected_playlist.as_ref().unwrap()).unwrap().id,
                    match &data_files_menu_data.playlist_entries {
                        MyFileEntry::Valid(entries) => {
                            //println!("доступно в списке: {:?}", entries.parsed_value);
                            &entries.parsed_value
                        },
                        _ => unreachable!(),
                    },
                    &mut self.data.playlist_test_error_log
                );
                //println!("Найдены id: {:?}", audio_ids);
                self.data.audio_in_playlist = get_audio_entries_from_ids(&audio_ids, &base_menu_data, &data_files_menu_data, &mut self.data.playlist_test_error_log);
                if !self.data.audio_in_playlist.is_empty() {
                    self.data.is_validation_required = false;
                }
                //println!("Найдены песни({}): {:?}", self.data.audio_in_playlist.len(), self.data.audio_in_playlist);
                //println!("пести validation_required: {}", self.data.is_validation_required);
            },
            MyMessage::Export => {
                for audio in &self.data.audio_in_playlist {
                    let audio_path = std::path::Path::new(&audio.path);
                    let dest_path = {
                        let mut p = self.data.output_path.clone();
                        p.push(audio_path.file_name().unwrap());
                        p
                    };
                    println!("{} => {}", audio.path, dest_path.to_string_lossy());
                }
                self.data.is_exported = true;
            },
            MyMessage::OpenSelectionMenu => {
                self.data.is_opened = true;
            },
            MyMessage::CloseSelectionMenu => {
                self.data.is_opened = false;
            },
            _ => (),
        }
    }
}

type AudioEntryId = u32;

// Перемещается назад, идя вперёд на N-i позиций
// Да, это сумашествие, но Path лишён ExactSizeIterator, а итерироваться по
// пути доверенным образом хочется
struct MyPathRevIterator<'a> {
    path_it: std::iter::Enumerate<std::path::Iter<'a>>,
    current_position_plus_1: usize,
}
impl<'a> Iterator for MyPathRevIterator<'a> {
    type Item = (usize, &'a std::ffi::OsStr);
    fn next(&mut self) -> Option<(usize, &'a std::ffi::OsStr)> {
        //println!("i to skip = {}", self.current_position_plus_1);
        if 0 == self.current_position_plus_1 {
            None
        }
        else {
            self.current_position_plus_1 = self.current_position_plus_1 - 1;
            return self.path_it.clone().skip(self.current_position_plus_1).next();
        }
    }
}
trait MyPathRevIteratorTrait<'a> {
    fn  my_rev_from_skip(self) -> MyPathRevIterator<'a>;
}
impl<'a> MyPathRevIteratorTrait<'a> for std::iter::Enumerate<std::path::Iter<'a>> {
    fn  my_rev_from_skip(self) -> MyPathRevIterator<'a> {
        let total = self.clone().count();
        MyPathRevIterator {
            path_it:                 self,
            current_position_plus_1: total,
        }
    }
}

fn path_from_db_to_real (path_from_db_string: &String, path_to_library: &PathBuf) -> Option<String> {
    let mut b = [0; 2];
    let path_from_db_string_copy = path_from_db_string.clone().replace('\\', std::path::MAIN_SEPARATOR.encode_utf8(&mut b));
    let path_from_db = std::path::Path::new(&path_from_db_string_copy);

    //println!("Исходные: path_from_db={:?}, path_to_library={:?}", std::path::Path::new(&path_from_db_string_copy).to_string_lossy(), path_to_library.to_string_lossy());
    //println!("len for db={}, len for lib={}", path_from_db.iter().count(), path_to_library.iter().count());

    let mut path_component_from_db_it = match path_from_db.parent(){
        Some(p) => p,
        _ => return None,
    }.iter().enumerate().my_rev_from_skip().skip(1);
    let dir_name_in_library_path = path_to_library.file_name().unwrap();

    while let Some((i, path_from_db_component)) = path_component_from_db_it.next() {
        if path_from_db_component == dir_name_in_library_path {
            let tail_it = path_from_db.iter().skip(1 + i);

            let dbg_tail = tail_it.clone().fold(PathBuf::new(), |mut accumulation, component|{accumulation.push(component);accumulation}).to_string_lossy().into_owned();
            let path_candidate = tail_it.fold(path_to_library.clone(), |mut accumulation, component| {accumulation.push(component); accumulation});

            //println!("tail: {}, canditate: {}", dbg_tail, path_candidate.to_string_lossy());
            if path_candidate.is_file() {
                return Some(path_candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn get_audio_entries_from_ids (audio_ids: &Vec<AudioEntryId>, base_menu_data: &BaseMenuData, data_files_menu_data: &DataFilesMenuData, playlist_test_error_log: &mut Vec<String>) -> Vec<AudioEntry> {
    let mut result = Vec::new();
    let connection = match rusqlite::Connection::open_with_flags(match &data_files_menu_data.audio_database {
        MyFileEntry::Valid(filepath) => &filepath.0,
        _ => unreachable!(),
    }, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => {
            playlist_test_error_log.push("# Ошибка: повторное открытие файла базы данных не удалось".into());
            return Vec::new();
        }
    };
    let mut statement = connection.prepare("SELECT ft_title, ft_artist, ft_path FROM file_t WHERE ft_id = ?1").unwrap();
    for audio_id in audio_ids {
        let output_rows = match statement.query_map(&[audio_id], |row| {
            Ok((row.get("ft_title")?, row.get("ft_artist")?, row.get("ft_path")?))
        }) {
            Ok(rows) => rows,
            Err(_) => {
                playlist_test_error_log.push(format!("# Ошибка при формировании запроса к базе данных на id={}", audio_id));
                continue;
            }
        };
        let mut succeed = false;
        let mut used_path = String::new();
        for (i, extraction_res) in output_rows.enumerate() {
            match extraction_res {
                Ok((raw_title, raw_artist, raw_path)) => {
                    //println!("\n\n# Начало подгона");
                    if let Some(true_path) = path_from_db_to_real(&raw_path, &base_menu_data.library_path) {
                        if succeed {
                            playlist_test_error_log.push(format!("# Замечание: в базе обнаружено {}-е вхождение audio_id={} ('{}') с путём '{}', используемое вхождение было раньше и вело к '{}'", i, audio_id, raw_title, true_path, used_path));
                            continue;
                        }
                        else {
                            used_path = true_path.clone();
                            result.push(AudioEntry{
                                title:  raw_title,
                                artist: raw_artist,
                                path:   true_path
                            });
                            succeed = true;
                        }
                    }
                    else {
                        if succeed {
                            playlist_test_error_log.push(format!("# Замечание: в базе обнаружено {}-е НЕвалидное вхождение audio_id={} ('{}') (ошибка исправления подгона под окружение пути '{}')", audio_id, i, raw_title, raw_path));
                        }
                        else {
                            playlist_test_error_log.push(format!("# Ошибка при при подгоне под окружение пути '{}' для audio_id={} ('{}'), {}-го результата", raw_path, raw_title, audio_id, i));
                        }
                    }
                },
                Err(_) => {
                    if succeed {
                        playlist_test_error_log.push(format!("# Замечание: в базе обнаружено {}-е НЕвалидное вхождение audio_id={} (ошибка при извлечении результата из базы данных)", audio_id, i));
                    }
                    else {
                        playlist_test_error_log.push(format!("# Ошибка при извлечении результата из базы данных для audio_id={}, {}-го результата", audio_id, i));
                    }
                }
            }
        }
    }
    result
}

fn get_entries_ids_for_playlist (playlist_id: &String, playlist_entries_array: &Vec<serde_json::Value>, playlist_test_error_log: &mut Vec<String>) -> Vec<AudioEntryId> {
    let mut result = Vec::new();
    for arr_entry in playlist_entries_array {
        match arr_entry {
            serde_json::Value::Object(obj) => {
                match obj.get("payload") {
                    Some(payload_value) => match payload_value {
                        serde_json::Value::String(payload_str) => {
                            if let Ok(payload_obj) = serde_json::from_str::<serde_json::Value>(payload_str.as_str()) {


                                match (payload_obj.get("PlaylistId"), payload_obj.get("LocalId")) {
                                    (Some(playlist_id_value), Some(audio_id_value)) => match (serde_json::from_value::<String>(playlist_id_value.clone()), serde_json::from_value(audio_id_value.clone())){
                                        (Ok(playlist_id_string), Ok(audio_id_number)) => if &playlist_id_string == playlist_id {
                                            result.push(audio_id_number);
                                        },
                                        _ => {
                                            playlist_test_error_log.push(
                                                format!("# Ошибка: неожиданный формат JSON в идентификаторах плейлиста или аудио-файла, ожидались строка и число, для плейлиста {}, для аудио-файла {}", get_error_str_for_json_entry(&playlist_id_value), get_error_str_for_json_entry(&audio_id_value))
                                            );
                                        }
                                    },
                                    _ => {
                                        playlist_test_error_log.push(
                                            "# Ошибка: не удалось найти в объекте содержимого плейтиста поля payload/PlaylistId и payload/LocalId".to_string()
                                        );
                                    }
                                }



                            }
                            else {
                                playlist_test_error_log.push(format!("# Ошибка: не удалось распознать строку в поле payload '{}' как объект JSON", payload_str));
                            }
                        },
                        _ => {
                            playlist_test_error_log.push("# Ошибка: поле payload в объекте содержимого плейлиста должно быть строкой".to_string());
                        }
                    },
                    _ => {
                        playlist_test_error_log.push(
                            "# Ошибка: не удалось найти в объекте содержимого плейтиста поле payload".to_string()
                        );
                    }
                }
            },
            _ => {
                playlist_test_error_log.push(
                    format!("# Ошибка: неожиданный формат JSON в массиве вхождений плейлиста, ожидался объект (таблица ключ-значение), {}", get_error_str_for_json_entry(arr_entry))
                )
            }
        }
    }
    result
}

struct MainAppIcedStates {
    scrl_menus: iced::scrollable::State,
}

struct MainApp {
    base_menu: BaseMenu,
    data_files_menu: DataFilesMenu,
    selection_menu: SelectionMenu,
    log: Log,

    states: MainAppIcedStates,
}

impl MainApp {
    fn new () -> MainApp {
        MainApp {
            base_menu:       BaseMenu::new(),
            data_files_menu: DataFilesMenu::new(),
            selection_menu:  SelectionMenu::new(),
            log:             Log::new(),
            states: MainAppIcedStates{
                scrl_menus: iced::scrollable::State::new(),
            }
        }
    }
}

#[derive(Clone,Debug,PartialEq)]
enum MyMessage {
    SetAudilasPath,
    SetLibraryPath,
    CloseBaseMenu,
    ValidateBaseMenu,
    OpenBaseMenu,

    SetPlaylistsPath,
    SetPlaylistEntriesPath,
    SetAudioDatabasePath,
    CloseDataFilesMenu,
    OpenDataFilesMenu,

    SelectPlaylist(String),
    TestPlaylist,
    Export,
    SetOutputPath,
    CloseSelectionMenu,
    OpenSelectionMenu,
}

fn gen_text (s: &str) -> iced::Text {
    iced::Text::new(s).color([0.0,0.0,0.0]).size(32)
}

struct Log {
    scrl_state: iced::scrollable::State,
}

impl Log {
    fn new () -> Log {
        Log {
            scrl_state: iced::scrollable::State::new()
        }
    }
    fn view_scroll (&mut self, base_menu_data: &BaseMenuData, data_files_menu_data: &DataFilesMenuData, selection_menu_data: &SelectionMenuData) -> iced::Scrollable<MyMessage> {
        let mut scroll = iced::Scrollable::new(&mut self.scrl_state);
        if !base_menu_data.validation_required {
            if data_files_menu_data.is_valid() {
                if selection_menu_data.is_exported {
                    scroll = scroll.push(gen_text("Готово!").color([0.0, 1.0, 0.0]));
                }
                for err in &selection_menu_data.playlist_test_error_log {
                    scroll = scroll.push(
                        gen_text(err.as_str()).color([1.0, 0.0, 0.0])
                    );
                }
                if !selection_menu_data.audio_in_playlist.is_empty() {
                    scroll = scroll
                        .push(iced::Row::new()
                            .push(gen_text("Элементов в плейлисте: "))
                            .push(gen_text(selection_menu_data.audio_in_playlist.len().to_string().as_str()))
                        )
                }
                if let Some(selected_playlist_string) = &selection_menu_data.selected_playlist {
                    scroll = scroll
                        .push(iced::Row::new()
                            .push(gen_text("Выбранный плейлист: "))
                            .push(gen_text(selected_playlist_string.as_str()))
                        );
                }
                scroll = scroll
                    .push(iced::Row::new()
                        .push(gen_text("Найдено плейлистов: "))
                        .push(gen_text(selection_menu_data.playlists.len().to_string().as_str()))
                    );
                for err in &selection_menu_data.playlists_error_log {
                    scroll = scroll.push(
                        gen_text(err.as_str()).color([1.0, 0.0, 0.0])
                    );
                }

                scroll = scroll
                    .push(iced::Row::new()
                        .push(gen_text("Папка для результатов: "))
                        .push(gen_text(selection_menu_data.output_path.to_string_lossy().into_owned().as_str()))
                    );
            }
            scroll = scroll
                .push(iced::Row::new()
                    .push(gen_text("Путь до базы данных с музыкой: "))
                    .push(data_files_menu_data.audio_database.to_iced_full_text())
                )
                .push(iced::Row::new()
                    .push(gen_text("Путь до файла со списком вхождений в плейлист: "))
                    .push(data_files_menu_data.playlist_entries.to_iced_full_text())
                )
                .push(iced::Row::new()
                    .push(gen_text("Путь до файла с плейлистами: "))
                    .push(data_files_menu_data.playlists.to_iced_full_text())
                );

            scroll = scroll
                .push(iced::Row::new()
                    .push(gen_text("Папка с библиотекой: "))
                    .push(gen_text(base_menu_data.library_path.to_string_lossy().into_owned().as_str()))
                )
                .push(iced::Row::new()
                    .push(gen_text("Папка с Audials: "))
                    .push(gen_text(base_menu_data.audilas_path.to_string_lossy().into_owned().as_str()))
                )
        }
        scroll
    }
}

impl iced::Application for MainApp {
    type Executor = iced::executor::Default;
    type Message = MyMessage;
    type Flags = ();

    fn new (_flags: ()) -> (MainApp, iced::Command<Self::Message>) {
        (MainApp::new(), iced::Command::none())
    }

    fn title (&self) -> String {
        String::from("Audials-playlist-exporter")
    }
    fn update (
        &mut self, message: Self::Message, _clipboard: &mut iced::Clipboard
    ) -> iced::Command<Self::Message> {
        self.base_menu.update_if_acceptable(&message);

        if !self.base_menu.data.validation_required && self.data_files_menu.data.is_initial() {
            self.data_files_menu.data.init_auto(&self.base_menu.data);
        }
        self.data_files_menu.update_if_acceptable(&message);

        if self.data_files_menu.data.is_valid() && !self.data_files_menu.data.is_valid_prev {
            self.selection_menu.data.init(&self.data_files_menu.data);
        }
        self.data_files_menu.data.is_valid_prev = self.data_files_menu.data.is_valid();
        self.selection_menu.update_if_acceptable(&message, &self.base_menu.data, &self.data_files_menu.data);

        if self.base_menu.data.validation_required {
            self.data_files_menu.reset_me();
            self.selection_menu.reset_me();
        }
        else if !self.data_files_menu.data.is_valid() {
            self.selection_menu.reset_me();
        }
        iced::Command::none()
    }
    fn view (&mut self) -> iced::Element<Self::Message> {
        let mut menus = iced::Scrollable::new(&mut self.states.scrl_menus);

        if self.data_files_menu.data.is_valid() {
            menus = menus.push(SelectionMenu::view(&self.selection_menu.data, &mut self.selection_menu.states));
        }

        if !self.base_menu.data.validation_required {
            //if self.data_files_menu.data.is_initial() {
            //    self.data_files_menu.data.init_auto(&self.base_menu.data);
            //}
            menus = menus.push(DataFilesMenu::view(&self.data_files_menu.data, &mut self.data_files_menu.states));
        }
        menus = menus.push(BaseMenu::view(&self.base_menu.data, &mut self.base_menu.states));

        let view = iced::Column::new();
        view.push::<iced::Element<MyMessage>>(menus.height(iced::Length::FillPortion(1)).width(iced::Length::Fill).into())
            .push(self.log.view_scroll(&self.base_menu.data, &self.data_files_menu.data, &self.selection_menu.data).height(iced::Length::FillPortion(1)).width(iced::Length::Fill))
            .into()
    }
}
