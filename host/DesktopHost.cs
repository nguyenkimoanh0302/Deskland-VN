// Deskland · Trình quản lý màn hình nền (Desktop Host)
//
// Các chức năng chính:
//   1. Gắn các cửa sổ không viền vào bên dưới các biểu tượng màn hình nền (lớp hình nền) trên tất cả màn hình, sử dụng WebView2 để hiển thị trang chủ đề hiện tại (như prototype/flat.html?wallpaper=1)
//   0. Cửa sổ cài đặt (prototype/settings.html): chọn chủ đề, chỉnh tham số, đồng bộ trực tiếp với hình nền và lưu vào settings.json; chia sẻ một chạm (vẽ ảnh hoặc quay video màn hình nền đã làm mờ biểu tượng)
//   2. Cứ mỗi 400ms đọc vị trí thực của các biểu tượng màn hình nền (qua thông điệp liên tiến trình của ListView), chỉ gửi tới trang khi có thay đổi
//   3. Tạm dừng hoạt ảnh trên từng màn hình khi có cửa sổ phóng to / toàn màn hình che màn hình đó; biểu tượng khay hệ thống cho phép tạm dừng, tải lại, thoát (khôi phục hình nền gốc khi thoát)
//   4. Khi Explorer dựng lại lớp hình nền (ví dụ nhấn Win+D), cửa sổ của ứng dụng bị hủy theo → tự động gắn lại
//
// Biên dịch bằng trình biên dịch .NET Framework tích hợp sẵn trên Windows (cú pháp C# 5), xem build.ps1.

using System;
using System.Diagnostics;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using System.Windows.Forms;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using Microsoft.Win32;

static class Program
{
    public static readonly string ExitEventName = "DesklandHost_Exit" + InstallProfile.Suffix;
    public static readonly string MenuEventName = "DesklandHost_Menu" + InstallProfile.Suffix;
    public static readonly string SettingsEventName = "DesklandHost_Settings" + InstallProfile.Suffix;

    // Thông báo cho tiến trình đang chạy (thoát / bật menu / mở cài đặt)
    static void Signal(string name)
    {
        Native.AllowSetForegroundWindow(-1);
        EventWaitHandle ev;
        if (EventWaitHandle.TryOpenExisting(name, out ev)) ev.Set();
    }

    [STAThread]
    static void Main(string[] args)
    {
        // Deskland.exe --exit : thông báo cho tiến trình đang chạy thoát bình thường (khôi phục hình nền gốc)
        // Deskland.exe --menu : bật menu tại vị trí con trỏ chuột
        // Deskland.exe --settings : mở cửa sổ cài đặt
        if (args.Length > 0 && args[0] == "--exit") { Signal(ExitEventName); return; }
        if (args.Length > 0 && args[0] == "--menu") { Signal(MenuEventName); return; }
        if (args.Length > 0 && args[0] == "--settings") { Signal(SettingsEventName); return; }
        if (args.Length > 0 && args[0] == "--exit-installed") { Environment.ExitCode = InstallProfile.StopInstalled(); return; }
        bool fromAutostart = Array.IndexOf(args, "--autostart") >= 0;
        if (fromAutostart) AutoStartCheck.MarkLaunched();
        bool created;
        var mutex = new Mutex(true, "DesklandHost_Singleton" + InstallProfile.Suffix, out created);
        if (!created)
        {
            if (!fromAutostart) Signal(SettingsEventName);
            return;
        }
        // Làm việc theo điểm ảnh vật lý (DPI-aware)
        if (!Native.SetProcessDpiAwarenessContext(new IntPtr(-4))) Native.SetProcessDPIAware();
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        // Ghi mọi ngoại lệ vào nhật ký
        Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
        Application.ThreadException += delegate(object s, ThreadExceptionEventArgs e) { Log.Write("Ngoại lệ chưa xử lý: " + e.Exception); };
        AppDomain.CurrentDomain.UnhandledException += delegate(object s, UnhandledExceptionEventArgs e) { Log.Write("Ngoại lệ nghiêm trọng: " + e.ExceptionObject); };
        string exeDir = AppDomain.CurrentDomain.BaseDirectory;
        string pageDir = Path.GetFullPath(Path.Combine(exeDir, @"..\..\prototype"));
        // Chế độ gắn trên Windows 11 24H2+: mặc định là cửa sổ con phân lớp của Progman
        string attach = "progman";
        bool autostart = false;
        foreach (string a in args)
        {
            if (a == "--legacy") attach = "legacy";
            else if (a == "--defview") attach = "defview";
            else if (a == "--autostart") autostart = true;
            else if (!a.StartsWith("--")) pageDir = a;
        }
        WallpaperForm.AttachMode = attach;
        Application.Run(new HostContext(pageDir, !autostart));
        GC.KeepAlive(mutex);
    }
}

static class Log
{
    public static readonly string DataDir = InstallProfile.DataDir;
    public static readonly string FilePath = Path.Combine(DataDir, "host.log");

    public static void Write(string msg)
    {
        try
        {
            Directory.CreateDirectory(DataDir);
            File.AppendAllText(FilePath, DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss ") + msg + "\r\n");
        }
        catch { }
    }
}

// Chuyển đổi tên thư mục dữ liệu cũ (HabitatWallpaper → Deskland) nếu có
static class Rename
{
    const string OldName = "HabitatWallpaper";
    public static void Migrate()
    {
        string oldDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), OldName);
        try
        {
            if (Directory.Exists(oldDir) && !Directory.Exists(Log.DataDir))
            {
                try { Directory.Move(oldDir, Log.DataDir); }
                catch
                {
                    Directory.CreateDirectory(Log.DataDir);
                    foreach (string src in Directory.GetFiles(oldDir))
                        if (!src.EndsWith(".log", StringComparison.OrdinalIgnoreCase)) File.Copy(src, Path.Combine(Log.DataDir, Path.GetFileName(src)), false);
                }
                Log.Write("Đã di chuyển thư mục dữ liệu từ " + oldDir + " sang " + Log.DataDir);
            }
        }
        catch (Exception ex) { Log.Write("Di chuyển thư mục dữ liệu thất bại: " + ex.Message); }
        try
        {
            using (RegistryKey k = Registry.CurrentUser.OpenSubKey(HostContext.RunKey, true))
            {
                if (k != null && k.GetValue(OldName) != null)
                {
                    k.DeleteValue(OldName, false);
                    k.SetValue(HostContext.RunName, "\"" + Application.ExecutablePath + "\" --autostart");
                    Log.Write("Đã cập nhật mục khởi động cùng hệ thống từ " + OldName + " sang " + HostContext.RunName);
                }
            }
        }
        catch (Exception ex) { Log.Write("Cập nhật mục khởi động cùng hệ thống thất bại: " + ex.Message); }
    }
}

// Kiểm tra xem khởi động cùng hệ thống có bị chặn hay không
static class AutoStartCheck
{
    static string LastPath { get { return Path.Combine(Log.DataDir, "autostart.last"); } }
    static string EnabledPath { get { return Path.Combine(Log.DataDir, "autostart.enabled"); } }

    static void Stamp(string path)
    {
        try { Directory.CreateDirectory(Log.DataDir); File.WriteAllText(path, DateTime.UtcNow.Ticks.ToString()); } catch { }
    }
    static DateTime ReadStamp(string path)
    {
        try { return new DateTime(long.Parse(File.ReadAllText(path).Trim()), DateTimeKind.Utc); } catch { return DateTime.MinValue; }
    }
    public static void MarkLaunched() { Stamp(LastPath); }
    public static void MarkEnabled() { Stamp(EnabledPath); }

    // Thời điểm đăng nhập phiên hiện tại (UTC)
    public static DateTime LogonTimeUtc()
    {
        IntPtr buf; int bytes;
        if (!Native.WTSQuerySessionInformation(IntPtr.Zero, -1, 24, out buf, out bytes)) return DateTime.MinValue;
        try { return bytes >= 216 ? DateTime.FromFileTimeUtc(Marshal.ReadInt64(buf, 200)) : DateTime.MinValue; }
        catch { return DateTime.MinValue; }
        finally { Native.WTSFreeMemory(buf); }
    }

    public static string SecuritySoftware()
    {
        return "";
    }

    public static string Detect(bool autoStartOn)
    {
        if (!autoStartOn) return null;
        DateTime logon = LogonTimeUtc();
        if (logon == DateTime.MinValue || (DateTime.UtcNow - logon).TotalMinutes < 3) return null;
        if (ReadStamp(EnabledPath) > logon) return null;
        if (ReadStamp(LastPath) > logon) return null;
        string sw = SecuritySoftware();
        Log.Write("Khởi động cùng hệ thống không chạy trong lần đăng nhập này (đăng nhập lúc " + logon.ToLocalTime().ToString("yyyy-MM-dd HH:mm") + ")");
        return sw;
    }
}

static class Json
{
    static readonly JavaScriptSerializer S = new JavaScriptSerializer();
    public static object Parse(string s) { return S.DeserializeObject(s); }
    public static string Stringify(object o) { return S.Serialize(o); }
    public static Dictionary<string, object> Obj(params object[] kv)
    {
        var d = new Dictionary<string, object>();
        for (int i = 0; i + 1 < kv.Length; i += 2) d[(string)kv[i]] = kv[i + 1];
        return d;
    }
}

// Cài đặt người dùng: chủ đề hiện tại + tham số của từng chủ đề
// Lưu tại %LOCALAPPDATA%\DesklandPreview\settings.json
static class Settings
{
    static readonly string FilePath = Path.Combine(Log.DataDir, "settings.json");
    public static string Theme = "beach";
    public static string Lang = "auto";
    public static bool LockWidget = true;
    public static int LockIdleMins = 5;
    public static bool LockRequireAuth = true;
    public static bool LockSyncWin = true;
    public static bool LockOverrideWinL = true;
    public static string LockPin = "";
    public static Dictionary<string, Dictionary<string, object>> Params = new Dictionary<string, Dictionary<string, object>>();

    public static void Load()
    {
        try
        {
            if (!File.Exists(FilePath)) return;
            var root = Json.Parse(File.ReadAllText(FilePath, Encoding.UTF8)) as Dictionary<string, object>;
            if (root == null) return;
            object v;
            if (root.TryGetValue("theme", out v) && v is string) Theme = (string)v;
            if (root.TryGetValue("lang", out v) && v is string) Lang = (string)v;
            if (root.TryGetValue("lockWidget", out v) && v is bool) LockWidget = (bool)v;
            if (root.TryGetValue("lockIdleMins", out v) && v is int) LockIdleMins = (int)v;
            if (root.TryGetValue("lockRequirePass", out v) && v is bool) LockRequireAuth = (bool)v;
            if (root.TryGetValue("lockSyncWin", out v) && v is bool) LockSyncWin = (bool)v;
            if (root.TryGetValue("lockOverrideWinL", out v) && v is bool) LockOverrideWinL = (bool)v;
            if (root.TryGetValue("lockPin", out v) && v is string) LockPin = (string)v;
            if (root.TryGetValue("params", out v) && v is Dictionary<string, object>)
                foreach (var kv in (Dictionary<string, object>)v)
                {
                    var d = kv.Value as Dictionary<string, object>;
                    if (d != null) Params[kv.Key] = d;
                }
            Log.Write("Đã đọc cài đặt: chủ đề " + Theme);
        }
        catch (Exception ex) { Log.Write("Đọc cài đặt thất bại (sẽ dùng mặc định): " + ex.Message); }
    }

    public static void Save()
    {
        try
        {
            Directory.CreateDirectory(Log.DataDir);
            File.WriteAllText(FilePath, Json.Stringify(Json.Obj(
                "version", 1, "theme", Theme, "lang", Lang,
                "lockWidget", LockWidget, "lockIdleMins", LockIdleMins,
                "lockRequirePass", LockRequireAuth, "lockSyncWin", LockSyncWin,
                "lockOverrideWinL", LockOverrideWinL, "lockPin", LockPin ?? "",
                "params", Params)), new UTF8Encoding(false));
        }
        catch (Exception ex) { Log.Write("Lưu cài đặt thất bại: " + ex.Message); }
    }

    public static Dictionary<string, object> For(string theme)
    {
        Dictionary<string, object> d;
        if (!Params.TryGetValue(theme, out d)) Params[theme] = d = new Dictionary<string, object>();
        return d;
    }
}

// Ngôn ngữ giao diện: bảng chuỗi nằm tại prototype/i18n/<ngôn_ngữ>.json (vi, en).
static class I18n
{
    public static readonly string[] Supported = { "vi", "en" };
    public static string Code = "vi", Sys = "vi";
    static Dictionary<string, object> cur, en, vi;

    public static string FromCulture(System.Globalization.CultureInfo c)
    {
        string two = c.TwoLetterISOLanguageName.ToLowerInvariant();
        if (two == "vi") return "vi";
        if (Array.IndexOf(Supported, two) >= 0) return two;
        return "en";
    }

    public static void Load(string pageDir, string setting)
    {
        Sys = FromCulture(System.Globalization.CultureInfo.CurrentUICulture);
        Code = Array.IndexOf(Supported, setting) >= 0 ? setting : Sys;
        vi = Read(pageDir, "vi");
        en = Read(pageDir, "en");
        cur = Code == "vi" ? vi : Code == "en" ? en : Read(pageDir, Code);
        Log.Write("Ngôn ngữ giao diện: " + Code + " (cài đặt: " + setting + ", hệ thống: " + Sys + ")");
    }

    static Dictionary<string, object> Read(string pageDir, string code)
    {
        try
        {
            var root = Json.Parse(File.ReadAllText(Path.Combine(Path.Combine(pageDir, "i18n"), code + ".json"), Encoding.UTF8)) as Dictionary<string, object>;
            object h;
            return root != null && root.TryGetValue("host", out h) ? h as Dictionary<string, object> : null;
        }
        catch (Exception ex) { Log.Write("Đọc tệp ngôn ngữ thất bại (" + code + "): " + ex.Message); return null; }
    }

    // Lấy chuỗi dịch: ngôn ngữ hiện tại → Tiếng Anh → Tiếng Việt
    public static string T(string key, params string[] kv)
    {
        object v = null;
        bool found = (cur != null && cur.TryGetValue(key, out v)) || (en != null && en.TryGetValue(key, out v)) || (vi != null && vi.TryGetValue(key, out v));
        string str = found ? (v as string ?? key) : key;
        for (int i = 0; i + 1 < kv.Length; i += 2) str = str.Replace("{" + kv[i] + "}", kv[i + 1]);
        return str;
    }
}

// Vòng đời toàn bộ ứng dụng: biểu tượng khay hệ thống thường trú; tự động tạo lại cửa sổ hình nền nếu bị hủy
class HostContext : ApplicationContext
{
    public const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run", RunName = "DesklandPreview";
    static readonly MethodInfo ShowTrayMenu =
        typeof(NotifyIcon).GetMethod("ShowContextMenu", BindingFlags.Instance | BindingFlags.NonPublic);

    readonly string pageDir;
    readonly NotifyIcon tray;
    readonly MenuItem pauseItem, autoStartItem, settingsItem, lockItem, reloadItem, exitItem;
    readonly System.Windows.Forms.Timer watch;
    readonly EventWaitHandle exitEvent, menuEvent, settingsEvent;
    readonly HotkeyWindow hotkey = new HotkeyWindow();
    readonly WinLKeyboardHook winLHook = new WinLKeyboardHook();
    readonly Queue<DateTime> restarts = new Queue<DateTime>();
    readonly List<WallpaperForm> forms = new List<WallpaperForm>();
    readonly List<WallpaperForm> deadForms = new List<WallpaperForm>();
    readonly List<LockScreenForm> lockForms = new List<LockScreenForm>();
    SettingsForm settingsWin;
    DateTime nextStart = DateTime.MaxValue, saveDue = DateTime.MaxValue, winLockSyncDue = DateTime.MaxValue;
    DateTime lastUnlockAt = DateTime.MinValue;
    bool manualPause, exiting, syncingWinLock, pendingWinLLock;

    WallpaperForm PrimaryForm
    {
        get
        {
            for (int i = 0; i < forms.Count; i++)
                if (forms[i].IsPrimary) return forms[i];
            return forms.Count > 0 ? forms[0] : null;
        }
    }
    string autoStartSw;
    string updateVersion, updateUrl, updateStatus = "", updateError = "";
    double updateProgress = -1;

    public HostContext(string pageDir, bool showSettings)
    {
        this.pageDir = pageDir;
        Log.Write("Khởi động, thư mục trang: " + pageDir + (showSettings ? "" : " (khởi động cùng hệ thống)"));
        if (showSettings) autoStartSw = AutoStartCheck.Detect(IsAutoStart());
        Settings.Load();
        I18n.Load(pageDir, Settings.Lang);
        exitEvent = new EventWaitHandle(false, EventResetMode.AutoReset, Program.ExitEventName);
        menuEvent = new EventWaitHandle(false, EventResetMode.AutoReset, Program.MenuEventName);
        settingsEvent = new EventWaitHandle(false, EventResetMode.AutoReset, Program.SettingsEventName);

        // Phím tắt Ctrl+Alt+H: bật menu tại vị trí chuột; Ctrl+Alt+L & Win+L: bật màn hình khóa Deskland
        bool hotkeyOk = hotkey.Register(1, Native.MOD_CONTROL | Native.MOD_ALT | Native.MOD_NOREPEAT, Keys.H);
        bool lockKeyOk = hotkey.Register(2, Native.MOD_CONTROL | Native.MOD_ALT | Native.MOD_NOREPEAT, Keys.L);
        hotkey.HotkeyPressed += delegate(int id)
        {
            if (id == 1) ShowMenu();
            else if (id == 2) ShowLockScreen();
        };
        winLHook.IsLocked = delegate { return lockForms.Count > 0; };
        winLHook.WinLPressed += delegate { pendingWinLLock = true; };
        winLHook.ApplyWinLPolicy(Settings.LockOverrideWinL);
        winLHook.Start();
        AppDomain.CurrentDomain.ProcessExit += delegate { try { winLHook.Dispose(); } catch { } };
        Log.Write(hotkeyOk ? "Đã đăng ký phím tắt Ctrl+Alt+H" : "Đăng ký phím tắt Ctrl+Alt+H thất bại (có thể đã bị ứng dụng khác chiếm)");
        Log.Write(lockKeyOk ? "Đã đăng ký phím tắt khóa màn hình Ctrl+Alt+L" : "Đăng ký phím tắt Ctrl+Alt+L thất bại");

        pauseItem = new MenuItem(I18n.T("trayPause"), delegate
        {
            manualPause = !manualPause;
            pauseItem.Checked = manualPause;
            for (int i = 0; i < forms.Count; i++) forms[i].SetManualPause(manualPause || lockForms.Count > 0);
        });
        autoStartItem = new MenuItem(I18n.T("trayAutostart"), delegate { SetAutoStart(!autoStartItem.Checked); });
        autoStartItem.Checked = IsAutoStart();
        if (autoStartItem.Checked && AutoStartValue() != "")
        {
            try
            {
                if (CreateAutoStartShortcut())
                {
                    DeleteRunAutoStart();
                    Log.Write("Đã chuyển khởi động cùng hệ thống sang thư mục Startup của người dùng");
                }
                else if (AutoStartValue().IndexOf("--autostart") < 0)
                    using (RegistryKey k = Registry.CurrentUser.CreateSubKey(RunKey))
                        k.SetValue(RunName, "\"" + Application.ExecutablePath + "\" --autostart");
            }
            catch (Exception ex) { Log.Write("Chuyển đổi khởi động cùng hệ thống thất bại, giữ nguyên mục cũ: " + ex.Message); }
        }
        settingsItem = new MenuItem(I18n.T("traySettings"), delegate { OpenSettings(); });
        settingsItem.DefaultItem = true;
        lockItem = new MenuItem(I18n.T("trayLock"), delegate { ShowLockScreen(); });

        tray = new NotifyIcon();
        tray.Icon = AppIcon.Create();
        tray.Text = "Deskland Preview";
        tray.ContextMenu = new ContextMenu(new[] {
            settingsItem,
            lockItem,
            pauseItem,
            reloadItem = new MenuItem(I18n.T("trayReload"), delegate { for (int i = 0; i < forms.Count; i++) forms[i].Reload(); }),
            autoStartItem,
            new MenuItem("-"),
            exitItem = new MenuItem(I18n.T("trayExit"), delegate { Exit(); }),
        });
        tray.MouseUp += delegate(object s, MouseEventArgs e) { if (e.Button == MouseButtons.Left) ShowMenu(); };
        tray.DoubleClick += delegate { OpenSettings(); };
        tray.Visible = true;
        ShowWelcomeOnce(hotkeyOk);

        SystemEvents.DisplaySettingsChanged += OnDisplaySettingsChanged;
        watch = new System.Windows.Forms.Timer();
        watch.Interval = 500;
        watch.Tick += delegate { Watch(); };
        watch.Start();
        StartForm();
        if (showSettings) OpenSettings();
    }

    void ApplyTrayTexts()
    {
        settingsItem.Text = I18n.T("traySettings");
        lockItem.Text = I18n.T("trayLock");
        pauseItem.Text = I18n.T("trayPause");
        reloadItem.Text = I18n.T("trayReload");
        autoStartItem.Text = I18n.T("trayAutostart");
        exitItem.Text = I18n.T("trayExit");
    }

    // ---------------------------------------------------------------- Cửa sổ cài đặt
    public void OpenSettings()
    {
        if (exiting) return;
        if (settingsWin == null || settingsWin.IsDisposed)
        {
            settingsWin = new SettingsForm(pageDir, this);
            settingsWin.FormClosed += delegate { settingsWin = null; };
            Log.Write("Mở cửa sổ cài đặt");
        }
        if (settingsWin.WindowState == FormWindowState.Minimized) settingsWin.WindowState = FormWindowState.Normal;
        settingsWin.Show();
        Native.ShowWindow(settingsWin.Handle, Native.SW_RESTORE);
        settingsWin.Activate();
    }

    string SettingsMessage()
    {
        return Json.Stringify(Json.Obj("t", "settings", "v", Settings.For(Settings.Theme)));
    }

    string UpdateStateMessage()
    {
        return Json.Stringify(Json.Obj("t", "updateState", "version", updateVersion,
            "status", updateStatus, "progress", updateProgress, "error", updateError));
    }

    void PostUpdateState()
    {
        if (settingsWin != null) settingsWin.Post(UpdateStateMessage());
    }

    public void OfferUpdate(string version, string url)
    {
        Uri uri;
        if (string.IsNullOrWhiteSpace(version) || !Uri.TryCreate(url, UriKind.Absolute, out uri) || uri.Scheme != Uri.UriSchemeHttps)
            throw new ArgumentException("Thông tin cập nhật cần số phiên bản và địa chỉ HTTPS hợp lệ");
        updateVersion = version.Trim();
        updateUrl = uri.AbsoluteUri;
        updateStatus = "available";
        updateProgress = -1;
        updateError = "";
        PostUpdateState();
    }

    void SaveSoon() { saveDue = DateTime.Now.AddMilliseconds(600); }

    bool ThemeAllowed(string id)
    {
        return Themes.Allowed(pageDir, id, delegate(string product) { return false; });
    }

    // Xử lý thông điệp từ trang cài đặt: hello / set / reset / theme / autostart / lang / updateProgram
    public void OnSettingsMessage(string s)
    {
        var m = Json.Parse(s) as Dictionary<string, object>;
        if (m == null || settingsWin == null) return;
        object t;
        m.TryGetValue("t", out t);
        switch (t as string)
        {
            case "hello":
            {
                Rectangle scr = Screen.PrimaryScreen.Bounds;
                settingsWin.Post(Json.Stringify(Json.Obj("t", "init", "theme", Settings.Theme, "params", Settings.Params,
                    "screen", Json.Obj("w", scr.Width, "h", scr.Height), "autostart", IsAutoStart(),
                    "autostartWarn", autoStartSw != null, "autostartSw", autoStartSw ?? "",
                    "lang", Settings.Lang, "langEff", I18n.Code, "langSys", I18n.Sys,
                    "lockCfg", Json.Obj(
                        "widget", Settings.LockWidget,
                        "idleMins", Settings.LockIdleMins,
                        "requireAuth", Settings.LockRequireAuth,
                        "syncWin", Settings.LockSyncWin,
                        "overrideWinL", Settings.LockOverrideWinL,
                        "pin", Settings.LockPin ?? ""))));
                if (updateVersion != null) PostUpdateState();
                WallpaperForm pf = PrimaryForm;
                if (pf != null && pf.LastIcons != "") settingsWin.Post(pf.LastIcons);
                break;
            }
            case "set":
            {
                string theme = m["theme"] as string, key = m["key"] as string;
                if (theme == null || key == null) return;
                Settings.For(theme)[key] = m["value"];
                if (theme == Settings.Theme)
                {
                    string msg = Json.Stringify(Json.Obj("t", "settings", "v", Json.Obj(key, m["value"])));
                    for (int i = 0; i < forms.Count; i++) forms[i].Post(msg);
                    if (Settings.LockSyncWin) ScheduleWinLockSync(2200);
                }
                SaveSoon();
                break;
            }
            case "reset":
            {
                string theme = m["theme"] as string;
                if (theme == null) return;
                Settings.Params.Remove(theme);
                if (theme == Settings.Theme)
                {
                    for (int i = 0; i < forms.Count; i++) forms[i].Post("{\"t\":\"reset\"}");
                    if (Settings.LockSyncWin) ScheduleWinLockSync(2200);
                }
                SaveSoon();
                Log.Write("Khôi phục cài đặt mặc định: " + theme);
                break;
            }
            case "theme":
            {
                string id = m["id"] as string;
                if (id == null || !ThemeAllowed(id))
                {
                    settingsWin.Post(Json.Stringify(Json.Obj("t", "themeChanged", "id", Settings.Theme)));
                    Log.Write("Từ chối chuyển chủ đề không xác định: " + (id ?? "null"));
                    return;
                }
                string page = Themes.PageFor(pageDir, id);
                if (page == null || id == Settings.Theme) return;
                Settings.Theme = id;
                SaveSoon();
                for (int i = 0; i < forms.Count; i++) forms[i].LoadPage(page);
                if (Settings.LockSyncWin) ScheduleWinLockSync(2500);
                Log.Write("Chuyển chủ đề: " + id);
                break;
            }
            case "lockNow":
                ShowLockScreen();
                break;
            case "lockCfg":
            {
                object v;
                if (m.TryGetValue("widget", out v) && v is bool) Settings.LockWidget = (bool)v;
                if (m.TryGetValue("idleMins", out v) && v is int) Settings.LockIdleMins = Math.Max(0, (int)v);
                if (m.TryGetValue("requireAuth", out v) && v is bool) Settings.LockRequireAuth = (bool)v;
                if (m.TryGetValue("overrideWinL", out v) && v is bool)
                {
                    Settings.LockOverrideWinL = (bool)v;
                    winLHook.ApplyWinLPolicy(Settings.LockOverrideWinL);
                }
                if (m.TryGetValue("pin", out v) && v is string) Settings.LockPin = ((string)v).Trim();
                if (m.TryGetValue("syncWin", out v) && v is bool)
                {
                    bool prev = Settings.LockSyncWin;
                    Settings.LockSyncWin = (bool)v;
                    if (Settings.LockSyncWin && !prev) ScheduleWinLockSync(800);
                }
                SaveSoon();
                break;
            }
            case "syncWinLock":
                ScheduleWinLockSync(100);
                break;
            case "autostartWarnSeen":
                autoStartSw = null;
                break;
            case "lang":
            {
                object v;
                if (!m.TryGetValue("v", out v) || !(v is string)) return;
                Settings.Lang = (string)v;
                SaveSoon();
                I18n.Load(pageDir, Settings.Lang);
                ApplyTrayTexts();
                settingsWin.Post(Json.Stringify(Json.Obj("t", "lang", "v", Settings.Lang, "eff", I18n.Code, "sys", I18n.Sys)));
                break;
            }
            case "share":
            {
                WallpaperForm pf = PrimaryForm;
                if (pf == null) { PostShareResult(false, null, I18n.T("errNotReady")); break; }
                pf.RequestSnapshot(OnShareImage);
                break;
            }
            case "shareVideo":
            {
                WallpaperForm pf = PrimaryForm;
                if (pf == null) { PostShareResult(false, null, I18n.T("errNotReady")); break; }
                pf.RequestRecording(OnShareVideo);
                break;
            }
            case "openShare":
                if (lastShare != null && File.Exists(lastShare)) Process.Start(lastShare);
                break;
            case "openWebsite":
                Process.Start("https://deskland.app/");
                break;
            case "revealShare":
                if (lastShare != null && File.Exists(lastShare)) Process.Start("explorer.exe", "/select,\"" + lastShare + "\"");
                break;
            case "updateProgram":
                if (updateUrl != null) Process.Start(updateUrl);
                break;
            case "autostart":
                SetAutoStart(m["v"] is bool && (bool)m["v"]);
                settingsWin.Post(Json.Stringify(Json.Obj("t", "autostart", "v", IsAutoStart())));
                break;
        }
    }

    // ---- Chia sẻ ảnh: lưu vào "Pictures\Deskland Preview", sao chép vào clipboard và gửi ảnh thu nhỏ về trang cài đặt ----
    string lastShare;
    void OnShareImage(string thumb, string png, string err)
    {
        if (err != null || png == null) { PostShareResult(false, null, err ?? I18n.T("errNoImage")); Log.Write("Tạo ảnh chia sẻ thất bại: " + err); return; }
        try
        {
            int comma = png.IndexOf(',');
            byte[] bytes = Convert.FromBase64String(comma >= 0 ? png.Substring(comma + 1) : png);
            string dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "Deskland Preview");
            Directory.CreateDirectory(dir);
            string path = Path.Combine(dir, "Deskland " + DateTime.Now.ToString("yyyy-MM-dd HHmmss") + ".png");
            File.WriteAllBytes(path, bytes);
            lastShare = path;
            try
            {
                using (var ms = new MemoryStream(bytes))
                using (var img = Image.FromStream(ms))
                {
                    var data = new DataObject();
                    data.SetImage(new Bitmap(img));
                    data.SetData("PNG", false, new MemoryStream(bytes));
                    Clipboard.SetDataObject(data, true);
                }
            }
            catch (Exception ex) { Log.Write("Sao chép ảnh chia sẻ vào clipboard thất bại: " + ex.Message); }
            Log.Write("Đã tạo ảnh chia sẻ: " + path);
            if (settingsWin != null)
                settingsWin.Post(Json.Stringify(Json.Obj("t", "shareResult", "ok", true, "thumb", thumb, "path", path)));
        }
        catch (Exception ex)
        {
            Log.Write("Lưu ảnh chia sẻ thất bại: " + ex);
            PostShareResult(false, null, ex.Message);
        }
    }
    void PostShareResult(bool ok, string path, string msg)
    {
        if (settingsWin != null) settingsWin.Post(Json.Stringify(Json.Obj("t", "shareResult", "ok", ok, "path", path, "msg", msg)));
    }
    // Chia sẻ hoạt ảnh: lưu thành MP4 (hoặc WebM), đặt tệp vào clipboard
    void OnShareVideo(string mime, string poster, string video, string err)
    {
        if (err != null || video == null) { PostShareResult(false, null, err ?? I18n.T("errNoVideo")); Log.Write("Quay hoạt ảnh chia sẻ thất bại: " + err); return; }
        try
        {
            int comma = video.IndexOf(',');
            byte[] bytes = Convert.FromBase64String(comma >= 0 ? video.Substring(comma + 1) : video);
            string dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "Deskland Preview");
            Directory.CreateDirectory(dir);
            string name = "Deskland " + DateTime.Now.ToString("yyyy-MM-dd HHmmss") + (mime == "video/webm" ? ".webm" : ".mp4");
            string path = Path.Combine(dir, name);
            File.WriteAllBytes(path, bytes);
            lastShare = path;
            try
            {
                var files = new System.Collections.Specialized.StringCollection();
                files.Add(path);
                Clipboard.SetFileDropList(files);
            }
            catch (Exception ex) { Log.Write("Sao chép hoạt ảnh chia sẻ vào clipboard thất bại: " + ex.Message); }
            Log.Write("Đã quay hoạt ảnh chia sẻ: " + path + " (" + (bytes.Length / 1024) + " KB)");
            if (settingsWin != null)
            {
                settingsWin.MapShareFolder(dir);
                settingsWin.Post(Json.Stringify(Json.Obj("t", "shareResult", "ok", true, "kind", "video", "thumb", poster, "path", path,
                    "url", "https://share.local/" + Uri.EscapeDataString(name))));
            }
        }
        catch (Exception ex)
        {
            Log.Write("Lưu hoạt ảnh chia sẻ thất bại: " + ex);
            PostShareResult(false, null, ex.Message);
        }
    }

    void ShowMenu()
    {
        if (exiting) return;
        autoStartItem.Checked = IsAutoStart();
        if (ShowTrayMenu != null) ShowTrayMenu.Invoke(tray, null);
    }

    void ShowWelcomeOnce(bool hotkeyOk)
    {
        try
        {
            string flag = Path.Combine(Log.DataDir, "welcome.shown");
            if (File.Exists(flag)) return;
            Directory.CreateDirectory(Log.DataDir);
            tray.ShowBalloonTip(10000, I18n.T("welcomeTitle"),
                I18n.T("welcomeBody", "hotkey", hotkeyOk ? I18n.T("welcomeHotkey") : ""), ToolTipIcon.Info);
            File.WriteAllText(flag, DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss"));
        }
        catch (Exception ex) { Log.Write("Hiển thị thông báo chào mừng thất bại: " + ex.Message); }
    }

    static string AutoStartShortcutPath
    {
        get { return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Startup), "Deskland.lnk"); }
    }

    static object OpenAutoStartShortcut()
    {
        object shell = Activator.CreateInstance(Type.GetTypeFromProgID("WScript.Shell"));
        try
        {
            return shell.GetType().InvokeMember("CreateShortcut", BindingFlags.InvokeMethod,
                null, shell, new object[] { AutoStartShortcutPath });
        }
        finally { Marshal.FinalReleaseComObject(shell); }
    }

    static bool AutoStartShortcutEnabled()
    {
        if (!File.Exists(AutoStartShortcutPath)) return false;
        object link = null;
        try
        {
            link = OpenAutoStartShortcut();
            string target = link.GetType().InvokeMember("TargetPath", BindingFlags.GetProperty,
                null, link, null) as string;
            string arguments = link.GetType().InvokeMember("Arguments", BindingFlags.GetProperty,
                null, link, null) as string;
            return string.Equals(target, Application.ExecutablePath, StringComparison.OrdinalIgnoreCase)
                && arguments != null && arguments.IndexOf("--autostart", StringComparison.OrdinalIgnoreCase) >= 0;
        }
        catch { return false; }
        finally { if (link != null) Marshal.FinalReleaseComObject(link); }
    }

    static bool CreateAutoStartShortcut()
    {
        object link = null;
        try
        {
            link = OpenAutoStartShortcut();
            link.GetType().InvokeMember("TargetPath", BindingFlags.SetProperty,
                null, link, new object[] { Application.ExecutablePath });
            link.GetType().InvokeMember("Arguments", BindingFlags.SetProperty,
                null, link, new object[] { "--autostart" });
            link.GetType().InvokeMember("WorkingDirectory", BindingFlags.SetProperty,
                null, link, new object[] { Path.GetDirectoryName(Application.ExecutablePath) });
            link.GetType().InvokeMember("IconLocation", BindingFlags.SetProperty,
                null, link, new object[] { Application.ExecutablePath + ",0" });
            link.GetType().InvokeMember("Description", BindingFlags.SetProperty,
                null, link, new object[] { "Deskland" });
            link.GetType().InvokeMember("Save", BindingFlags.InvokeMethod, null, link, null);
            return File.Exists(AutoStartShortcutPath);
        }
        catch (Exception ex) { Log.Write("Tạo lối tắt trong thư mục Startup thất bại: " + ex.Message); return false; }
        finally { if (link != null) Marshal.FinalReleaseComObject(link); }
    }

    static string AutoStartValue()
    {
        if (InstallProfile.Isolated) return "";
        using (RegistryKey k = Registry.CurrentUser.OpenSubKey(RunKey))
        {
            string v = k == null ? null : k.GetValue(RunName) as string;
            return v ?? "";
        }
    }

    static void DeleteRunAutoStart()
    {
        using (RegistryKey k = Registry.CurrentUser.OpenSubKey(RunKey, true))
            if (k != null) k.DeleteValue(RunName, false);
    }

    static bool IsAutoStart()
    {
        return AutoStartShortcutEnabled()
            || AutoStartValue().IndexOf(Application.ExecutablePath, StringComparison.OrdinalIgnoreCase) >= 0;
    }

    void SetAutoStart(bool on)
    {
        if (InstallProfile.Isolated) return;
        try
        {
            if (on)
            {
                bool shortcut = CreateAutoStartShortcut();
                if (shortcut) DeleteRunAutoStart();
                else
                    using (RegistryKey k = Registry.CurrentUser.CreateSubKey(RunKey))
                        k.SetValue(RunName, "\"" + Application.ExecutablePath + "\" --autostart");
                AutoStartCheck.MarkEnabled();
                Log.Write(shortcut ? "Đã bật khởi động cùng hệ thống (thư mục Startup)" : "Đã bật khởi động cùng hệ thống (Registry Run)");
            }
            else
            {
                DeleteRunAutoStart();
                if (File.Exists(AutoStartShortcutPath)) File.Delete(AutoStartShortcutPath);
                Log.Write(IsAutoStart() ? "Tắt khởi động cùng hệ thống thất bại, mục khởi động vẫn tồn tại" : "Đã tắt khởi động cùng hệ thống");
            }
        }
        catch (Exception ex) { Log.Write("Thiết lập khởi động cùng hệ thống thất bại: " + ex.Message); }
        autoStartItem.Checked = IsAutoStart();
    }

    void StartForm()
    {
        nextStart = DateTime.MaxValue;
        ShutdownForms();
        if (!ThemeAllowed(Settings.Theme)) Settings.Theme = ThemeAllowed("beach") ? "beach" : "meadow";
        string page = ThemeAllowed(Settings.Theme) ? Themes.PageFor(pageDir, Settings.Theme) : null;
        if (page == null)
        {
            Log.Write("Không có chủ đề hợp lệ để tải, thử lại sau 5 giây");
            nextStart = DateTime.Now.AddSeconds(5);
            return;
        }
        Screen[] screens = Screen.AllScreens;
        if (screens == null || screens.Length == 0) screens = new[] { Screen.PrimaryScreen };
        bool primaryHooked = false;
        for (int i = 0; i < screens.Length; i++)
        {
            Screen scr = screens[i];
            bool isPrimary = scr.Primary || (!primaryHooked && i == screens.Length - 1);
            var f = new WallpaperForm(pageDir, page, manualPause, scr.Bounds, isPrimary);
            f.SettingsJson = SettingsMessage;
            if (isPrimary && !primaryHooked)
            {
                primaryHooked = true;
                f.IconsChanged += delegate(string json) { if (settingsWin != null) settingsWin.Post(json); };
                f.RecordProgress += delegate(string p) { if (settingsWin != null) settingsWin.Post("{\"t\":\"recProgress\",\"p\":" + p + "}"); };
                f.SceneReady += delegate { if (Settings.LockSyncWin) ScheduleWinLockSync(1800); };
            }
            f.HandleDestroyed += OnFormGone;
            forms.Add(f);
            f.Show();
        }
    }

    void ShutdownForms()
    {
        if (forms.Count == 0) return;
        WallpaperForm[] current = forms.ToArray();
        forms.Clear();
        for (int i = 0; i < current.Length; i++)
        {
            WallpaperForm f = current[i];
            f.HandleDestroyed -= OnFormGone;
            try { f.Shutdown(); } catch { }
            deadForms.Add(f);
        }
    }

    void OnDisplaySettingsChanged(object sender, EventArgs e)
    {
        if (exiting) return;
        Log.Write("Cấu hình màn hình thay đổi, gắn lại hình nền cho tất cả màn hình");
        ShutdownForms();
        nextStart = DateTime.Now.AddMilliseconds(600);
    }

    void OnFormGone(object sender, EventArgs e)
    {
        var f = (WallpaperForm)sender;
        if (!forms.Remove(f)) return;
        deadForms.Add(f);
        if (exiting) return;
        if (f.Fatal) { ShutdownForms(); Exit(); return; }
        ShutdownForms();

        restarts.Enqueue(DateTime.Now);
        while (restarts.Count > 0 && (DateTime.Now - restarts.Peek()).TotalSeconds > 60) restarts.Dequeue();
        if (restarts.Count > 5)
        {
            Log.Write("Cửa sổ hình nền bị hủy quá 5 lần trong 1 phút, dừng thử lại");
            MessageBox.Show(I18n.T("errRebuilt") + "\n\n" + I18n.T("errSeeLog", "path", Log.FilePath), "Deskland Preview");
            Exit();
            return;
        }
        Log.Write("Cửa sổ hình nền bị hủy (Explorer đã dựng lại lớp hình nền), gắn lại sau 1 giây");
        nextStart = DateTime.Now.AddSeconds(1);
    }

    // ---------------------------------------------------------------- Màn hình khóa động & Đồng bộ màn hình khóa Windows
    public void ShowLockScreen()
    {
        if (exiting || lockForms.Count > 0) return;
        string page = ThemeAllowed(Settings.Theme) ? Themes.PageFor(pageDir, Settings.Theme) : null;
        if (page == null) return;

        Log.Write("Bật màn hình khóa động Deskland (chủ đề: " + Settings.Theme + ")");
        for (int i = 0; i < forms.Count; i++) forms[i].SetManualPause(true);

        WallpaperForm pf = PrimaryForm;
        string iconsJson = pf != null ? pf.LastIcons : "";
        string settingsJson = SettingsMessage();

        Screen[] screens = Screen.AllScreens;
        if (screens == null || screens.Length == 0) screens = new[] { Screen.PrimaryScreen };
        bool primarySet = false;
        LockScreenForm primaryLock = null;
        for (int i = 0; i < screens.Length; i++)
        {
            Screen scr = screens[i];
            bool isPrimary = scr.Primary || (!primarySet && i == screens.Length - 1);
            if (isPrimary) primarySet = true;
            var lf = new LockScreenForm(pageDir, page, scr.Bounds, isPrimary, Settings.LockWidget, Settings.LockRequireAuth, Settings.LockPin, settingsJson, iconsJson, delegate
            {
                CloseLockScreen(true);
            });
            lockForms.Add(lf);
            lf.Show();
            if (isPrimary) primaryLock = lf;
        }
        if (primaryLock != null)
        {
            try { primaryLock.Activate(); } catch { }
        }
    }

    public void CloseLockScreen(bool userTriggered)
    {
        if (lockForms.Count == 0) return;
        LockScreenForm[] current = lockForms.ToArray();
        lockForms.Clear();
        lastUnlockAt = DateTime.Now;
        for (int i = 0; i < current.Length; i++)
        {
            try { current[i].Shutdown(); } catch { }
        }
        for (int i = 0; i < forms.Count; i++) forms[i].SetManualPause(manualPause);
        Log.Write("Đã mở khóa và đóng màn hình khóa động Deskland");
    }

    void ScheduleWinLockSync(int delayMs)
    {
        if (!Settings.LockSyncWin) return;
        winLockSyncDue = DateTime.Now.AddMilliseconds(delayMs);
    }

    void SyncWindowsLockScreen()
    {
        if (syncingWinLock || !Settings.LockSyncWin || exiting) return;
        WallpaperForm pf = PrimaryForm;
        if (pf == null) return;
        syncingWinLock = true;
        pf.RequestLockSnapshot(delegate(string png, string err)
        {
            if (err != null || string.IsNullOrEmpty(png))
            {
                syncingWinLock = false;
                Log.Write("Chụp ảnh đồng bộ màn hình khóa Windows thất bại: " + (err ?? "không có dữ liệu"));
                return;
            }
            ThreadPool.QueueUserWorkItem(delegate
            {
                try
                {
                    int comma = png.IndexOf(',');
                    byte[] bytes = Convert.FromBase64String(comma >= 0 ? png.Substring(comma + 1) : png);
                    Directory.CreateDirectory(Log.DataDir);
                    string imgPath = Path.Combine(Log.DataDir, "lockscreen.png");
                    File.WriteAllBytes(imgPath, bytes);

                    string psScript =
                        "Add-Type -AssemblyName System.Runtime.WindowsRuntime; " +
                        "$asTaskGen = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]; " +
                        "$asTaskAct = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]; " +
                        "[Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime] | Out-Null; " +
                        "[Windows.System.UserProfile.LockScreen,Windows.System.UserProfile,ContentType=WindowsRuntime] | Out-Null; " +
                        "$fTask = $asTaskGen.MakeGenericMethod([Windows.Storage.StorageFile]).Invoke($null, @([Windows.Storage.StorageFile]::GetFileFromPathAsync('" + imgPath.Replace("'", "''") + "'))); " +
                        "$file = $fTask.GetAwaiter().GetResult(); " +
                        "$sTask = $asTaskAct.Invoke($null, @([Windows.System.UserProfile.LockScreen]::SetImageFileAsync($file))); " +
                        "$sTask.GetAwaiter().GetResult();";

                    var psi = new ProcessStartInfo("powershell.exe", "-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"" + psScript.Replace("\"", "\\\"") + "\"")
                    {
                        CreateNoWindow = true,
                        UseShellExecute = false
                    };
                    using (Process p = Process.Start(psi))
                    {
                        if (p != null) p.WaitForExit(12000);
                    }
                    Log.Write("Đã đồng bộ ảnh cảnh hiện tại với màn hình khóa Windows (Win+L): " + imgPath);
                }
                catch (Exception ex)
                {
                    Log.Write("Đồng bộ màn hình khóa Windows thất bại: " + ex.Message);
                }
                finally
                {
                    syncingWinLock = false;
                }
            });
        });
    }

    void Watch()
    {
        if (deadForms.Count > 0)
        {
            for (int i = 0; i < deadForms.Count; i++)
                try { deadForms[i].Dispose(); } catch { }
            deadForms.Clear();
        }
        if (exitEvent.WaitOne(0)) { Exit(); return; }
        if (menuEvent.WaitOne(0)) ShowMenu();
        if (settingsEvent.WaitOne(0)) OpenSettings();
        if (pendingWinLLock) { pendingWinLLock = false; ShowLockScreen(); }
        if (DateTime.Now >= saveDue) { saveDue = DateTime.MaxValue; Settings.Save(); }
        if (DateTime.Now >= winLockSyncDue) { winLockSyncDue = DateTime.MaxValue; SyncWindowsLockScreen(); }
        if (forms.Count == 0 && !exiting && DateTime.Now >= nextStart) StartForm();

        if (!exiting && lockForms.Count == 0 && Settings.LockIdleMins > 0)
        {
            uint idleMs = Native.GetIdleMilliseconds();
            uint targetMs = (uint)(Settings.LockIdleMins * 60000);
            if (idleMs >= targetMs && (DateTime.Now - lastUnlockAt).TotalSeconds >= 10)
            {
                WallpaperForm pf = PrimaryForm;
                bool fullscreenBusy = pf != null && pf.IsCoveredByFullscreen && (settingsWin == null || !settingsWin.Visible);
                if (!fullscreenBusy) ShowLockScreen();
            }
        }
    }

    void Exit()
    {
        if (exiting) return;
        exiting = true;
        SystemEvents.DisplaySettingsChanged -= OnDisplaySettingsChanged;
        watch.Stop();
        CloseLockScreen(false);
        if (saveDue != DateTime.MaxValue) Settings.Save();
        if (settingsWin != null) { try { settingsWin.Close(); } catch { } }
        hotkey.Dispose();
        winLHook.Dispose();
        tray.Visible = false;
        ShutdownForms();
        Log.Write("Thoát ứng dụng");
        ExitThread();
    }
}

// Bộ bắt phím mức thấp (WH_KEYBOARD_LL) để chuyển hướng phím Win+L sang Màn hình khóa động Deskland
// và khóa các phím chuyển cửa sổ hệ thống khi màn hình khóa đang bật.
class WinLKeyboardHook : IDisposable
{
    const string PolicyKey = @"Software\Microsoft\Windows\CurrentVersion\Policies\System";
    const string PolicyValue = "DisableLockWorkstation";

    IntPtr hookId = IntPtr.Zero;
    Native.LowLevelKeyboardProc proc;
    bool winDown;
    public Func<bool> IsLocked;
    public event Action WinLPressed;

    public void Start()
    {
        if (hookId != IntPtr.Zero) return;
        proc = HookCallback;
        using (Process curProcess = Process.GetCurrentProcess())
        using (ProcessModule curModule = curProcess.MainModule)
        {
            hookId = Native.SetWindowsHookEx(13 /* WH_KEYBOARD_LL */, proc, Native.GetModuleHandle(curModule.ModuleName), 0);
        }
        Log.Write(hookId != IntPtr.Zero ? "Đã gắn bộ bắt phím Win+L (WH_KEYBOARD_LL)" : "Gắn bộ bắt phím WH_KEYBOARD_LL thất bại");
    }

    public void ApplyWinLPolicy(bool interceptWinL)
    {
        if (InstallProfile.Isolated) return;
        try
        {
            using (RegistryKey k = Registry.CurrentUser.CreateSubKey(PolicyKey))
            {
                if (k == null) return;
                if (interceptWinL)
                    k.SetValue(PolicyValue, 1, RegistryValueKind.DWord);
                else
                    k.DeleteValue(PolicyValue, false);
            }
            Log.Write(interceptWinL
                ? "Đã bật chế độ bắt phím Win+L cho màn hình khóa động Deskland"
                : "Đã khôi phục phím Win+L mặc định của Windows");
        }
        catch (Exception ex)
        {
            Log.Write("Cập nhật cấu hình phím Win+L thất bại: " + ex.Message);
        }
    }

    IntPtr HookCallback(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode >= 0)
        {
            int msg = wParam.ToInt32();
            bool keyDown = (msg == 0x0100 /* WM_KEYDOWN */ || msg == 0x0104 /* WM_SYSKEYDOWN */);
            bool keyUp = (msg == 0x0101 /* WM_KEYUP */ || msg == 0x0105 /* WM_SYSKEYUP */);
            Native.KBDLLHOOKSTRUCT kb = (Native.KBDLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(Native.KBDLLHOOKSTRUCT));
            int vk = (int)kb.vkCode;
            bool altDown = (kb.flags & 0x20 /* LLKHF_ALTDOWN */) != 0;

            if (vk == 0x5B /* VK_LWIN */ || vk == 0x5C /* VK_RWIN */)
            {
                if (keyDown) winDown = true;
                else if (keyUp) winDown = false;
            }

            bool winHeld = winDown || (Native.GetAsyncKeyState(0x5B) & 0x8000) != 0 || (Native.GetAsyncKeyState(0x5C) & 0x8000) != 0;

            // Khi nhấn Win + L và bật chế độ màn hình khóa động Deskland
            if (keyDown && vk == (int)Keys.L && winHeld && Settings.LockOverrideWinL)
            {
                if (WinLPressed != null) WinLPressed();
                return new IntPtr(1);
            }

            // Khi màn hình khóa động đang hiển thị, chặn các tổ hợp phím thoát cửa sổ hệ thống
            if (IsLocked != null && IsLocked())
            {
                if (vk == 0x5B || vk == 0x5C) return new IntPtr(1); // Chặn phím Windows
                if (altDown && (vk == (int)Keys.Tab || vk == (int)Keys.Escape || vk == (int)Keys.F4)) return new IntPtr(1); // Chặn Alt+Tab, Alt+Esc, Alt+F4
                if (vk == (int)Keys.Escape && (Native.GetAsyncKeyState(0x11 /* VK_CONTROL */) & 0x8000) != 0) return new IntPtr(1); // Chặn Ctrl+Esc
            }
        }
        return Native.CallNextHookEx(hookId, nCode, wParam, lParam);
    }

    public void Dispose()
    {
        if (hookId != IntPtr.Zero)
        {
            Native.UnhookWindowsHookEx(hookId);
            hookId = IntPtr.Zero;
        }
        ApplyWinLPolicy(false);
    }
}

// Cửa sổ ẩn nhận phím tắt toàn cục
class HotkeyWindow : NativeWindow, IDisposable
{
    public event Action<int> HotkeyPressed;
    readonly List<int> registeredIds = new List<int>();

    public HotkeyWindow()
    {
        var cp = new CreateParams();
        cp.Parent = new IntPtr(-3); // HWND_MESSAGE
        CreateHandle(cp);
    }

    public bool Register(int id, uint modifiers, Keys key)
    {
        bool ok = Native.RegisterHotKey(Handle, id, modifiers, (uint)key);
        if (ok) registeredIds.Add(id);
        return ok;
    }

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == 0x0312 /* WM_HOTKEY */ && HotkeyPressed != null)
            HotkeyPressed(m.WParam.ToInt32());
        base.WndProc(ref m);
    }

    public void Dispose()
    {
        for (int i = 0; i < registeredIds.Count; i++)
            Native.UnregisterHotKey(Handle, registeredIds[i]);
        registeredIds.Clear();
        if (Handle != IntPtr.Zero) DestroyHandle();
    }
}

// Biểu tượng dùng chung cho khay hệ thống và cửa sổ cài đặt
static class AppIcon
{
    public static Icon Create()
    {
        return new Icon(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "DesklandPreview.ico"));
    }

    static GraphicsPath RoundRect(Rectangle r, int radius)
    {
        int d = radius * 2;
        var p = new GraphicsPath();
        p.AddArc(r.X, r.Y, d, d, 180, 90);
        p.AddArc(r.Right - d, r.Y, d, d, 270, 90);
        p.AddArc(r.Right - d, r.Bottom - d, d, d, 0, 90);
        p.AddArc(r.X, r.Bottom - d, d, d, 90, 90);
        p.CloseFigure();
        return p;
    }
}

// Cửa sổ cài đặt hiển thị prototype/settings.html bằng WebView2
class SettingsForm : Form
{
    readonly WebView2 web;
    readonly HostContext host;
    readonly string pageDir;

    public SettingsForm(string pageDir, HostContext host)
    {
        this.pageDir = pageDir;
        this.host = host;
        Text = "Deskland Preview";
        AccessibleName = "Deskland Preview";
        ShowIcon = true;
        Icon = AppIcon.Create();
        StartPosition = FormStartPosition.CenterScreen;
        float s = Native.GetDpiForSystem() / 96f;
        ClientSize = new Size((int)(1120 * s), (int)(800 * s));
        MinimumSize = new Size((int)(940 * s), (int)(680 * s));
        BackColor = Color.White;

        web = new WebView2();
        web.Dock = DockStyle.Fill;
        web.DefaultBackgroundColor = BackColor;
        web.CreationProperties = new CoreWebView2CreationProperties();
        web.CreationProperties.UserDataFolder = Path.Combine(Log.DataDir, "WebView2");
        Controls.Add(web);
    }

    void ThemeTitleBar()
    {
        bool light = true;
        try
        {
            object v = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme", 1);
            light = v == null || Convert.ToInt32(v) != 0;
        }
        catch { }
        int dark = light ? 0 : 1;
        int color = light ? 0x00FFFFFF : 0x00171717;
        Native.DwmSetWindowAttribute(Handle, 20, ref dark, 4);
        Native.DwmSetWindowAttribute(Handle, 35, ref color, 4);
    }
    protected override void OnHandleCreated(EventArgs e) { base.OnHandleCreated(e); ThemeTitleBar(); }
    protected override void WndProc(ref Message m)
    {
        if (m.Msg == 0x001A /* WM_SETTINGCHANGE */) ThemeTitleBar();
        base.WndProc(ref m);
    }

    protected override async void OnShown(EventArgs e)
    {
        base.OnShown(e);
        try
        {
            await web.EnsureCoreWebView2Async(null);
            if (IsDisposed) return;
            var core = web.CoreWebView2;
            core.SetVirtualHostNameToFolderMapping("app.local", pageDir, CoreWebView2HostResourceAccessKind.Allow);
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.IsZoomControlEnabled = false;
            core.WebMessageReceived += delegate(object s, CoreWebView2WebMessageReceivedEventArgs a)
            {
                try { host.OnSettingsMessage(a.TryGetWebMessageAsString()); }
                catch (Exception ex) { Log.Write("Xử lý thông điệp cài đặt thất bại: " + ex.Message); }
            };
            string settingsUrl = "https://app.local/settings.html?v=" + Uri.EscapeDataString(Application.ProductVersion);
            Log.Write("Trang cài đặt: " + settingsUrl);
            core.Navigate(settingsUrl);
        }
        catch (Exception ex)
        {
            Log.Write("Khởi động cửa sổ cài đặt thất bại: " + ex);
            MessageBox.Show(I18n.T("errSettings", "msg", ex.Message), "Deskland Preview");
            Close();
        }
    }

    public void Post(string json)
    {
        try { if (!IsDisposed && web.CoreWebView2 != null) web.CoreWebView2.PostWebMessageAsString(json); }
        catch { }
    }

    bool shareMapped;
    public void MapShareFolder(string dir)
    {
        if (shareMapped || IsDisposed || web.CoreWebView2 == null) return;
        web.CoreWebView2.SetVirtualHostNameToFolderMapping("share.local", dir, CoreWebView2HostResourceAccessKind.Allow);
        shareMapped = true;
    }
}

// Cửa sổ Màn hình khóa động (Live Lock Screen & Widget Overlay) phủ trên toàn bộ các màn hình
class LockScreenForm : Form
{
    readonly WebView2 web;
    readonly System.Windows.Forms.Timer focusTimer;
    readonly string pageDir, page, lockPin, settingsJson, iconsJson;
    readonly bool isPrimary, showWidget, requireAuth;
    readonly Action onUnlock;
    bool shuttingDown;

    public LockScreenForm(string pageDir, string page, Rectangle bounds, bool isPrimary, bool showWidget, bool requirePassSetting, string lockPin, string settingsJson, string iconsJson, Action onUnlock)
    {
        this.pageDir = pageDir;
        this.page = page;
        this.isPrimary = isPrimary;
        this.showWidget = showWidget;
        this.lockPin = (lockPin ?? "").Trim();
        this.requireAuth = requirePassSetting || this.lockPin.Length > 0;
        this.settingsJson = settingsJson;
        this.iconsJson = iconsJson;
        this.onUnlock = onUnlock;

        FormBorderStyle = FormBorderStyle.None;
        StartPosition = FormStartPosition.Manual;
        Bounds = bounds;
        TopMost = true;
        ShowInTaskbar = false;
        KeyPreview = true;
        BackColor = Color.FromArgb(18, 28, 38);
        Text = "DesklandLockScreen";

        web = new WebView2();
        web.Dock = DockStyle.Fill;
        web.DefaultBackgroundColor = BackColor;
        web.CreationProperties = new CoreWebView2CreationProperties();
        web.CreationProperties.UserDataFolder = Path.Combine(Log.DataDir, "WebView2");
        Controls.Add(web);

        if (!this.requireAuth)
        {
            KeyDown += delegate { RequestUnlock(); };
            MouseDown += delegate { RequestUnlock(); };
        }
        else if (isPrimary)
        {
            focusTimer = new System.Windows.Forms.Timer();
            focusTimer.Interval = 600;
            focusTimer.Tick += delegate
            {
                if (shuttingDown || IsDisposed) return;
                try
                {
                    if (!TopMost) TopMost = true;
                    IntPtr fg = Native.GetForegroundWindow();
                    if (fg != Handle) Activate();
                }
                catch { }
            };
            focusTimer.Start();
        }
    }

    void RequestUnlock()
    {
        if (shuttingDown) return;
        if (onUnlock != null) onUnlock();
    }

    bool CheckCredential(string entered)
    {
        string val = entered ?? "";
        if (lockPin.Length > 0 && val == lockPin) return true;
        return Native.VerifyWindowsPassword(val, lockPin.Length == 0);
    }

    protected override async void OnShown(EventArgs e)
    {
        base.OnShown(e);
        try
        {
            await web.EnsureCoreWebView2Async(null);
            if (shuttingDown || IsDisposed) return;
            var core = web.CoreWebView2;
            core.SetVirtualHostNameToFolderMapping("app.local", pageDir, CoreWebView2HostResourceAccessKind.Allow);
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.IsZoomControlEnabled = false;
            core.WebMessageReceived += delegate(object s, CoreWebView2WebMessageReceivedEventArgs a)
            {
                string msg = null;
                try { msg = a.TryGetWebMessageAsString(); } catch { }
                if (msg == "unlock" && !requireAuth) { RequestUnlock(); return; }
                if (msg != null && msg.StartsWith("unlockPin|"))
                {
                    string entered = msg.Substring(10);
                    if (CheckCredential(entered)) RequestUnlock();
                    else
                    {
                        try { core.PostWebMessageAsString("{\"t\":\"pinError\"}"); } catch { }
                    }
                    return;
                }
                if (msg == "ready")
                {
                    try
                    {
                        if (!string.IsNullOrEmpty(settingsJson)) core.PostWebMessageAsString(settingsJson);
                        if (!string.IsNullOrEmpty(iconsJson)) core.PostWebMessageAsString(iconsJson);
                        core.PostWebMessageAsString("{\"t\":\"pause\",\"v\":false}");
                    }
                    catch { }
                }
            };
            string winUser = Environment.UserName ?? "User";
            string url = "https://app.local/" + page + "?wallpaper=1&lockscreen=1" +
                "&widget=" + (showWidget ? "1" : "0") +
                "&primary=" + (isPrimary ? "1" : "0") +
                "&pin=" + (requireAuth ? "1" : "0") +
                "&user=" + Uri.EscapeDataString(winUser) +
                "&lang=" + Uri.EscapeDataString(I18n.Code);
            core.Navigate(url);
        }
        catch (Exception ex)
        {
            Log.Write("Khởi tạo cửa sổ màn hình khóa thất bại: " + ex.Message);
            RequestUnlock();
        }
    }

    public void Shutdown()
    {
        if (shuttingDown) return;
        shuttingDown = true;
        if (focusTimer != null) { try { focusTimer.Stop(); focusTimer.Dispose(); } catch { } }
        try { web.Dispose(); } catch { }
        try { Close(); } catch { }
    }
}

class WallpaperForm : Form
{
    readonly WebView2 web;
    readonly System.Windows.Forms.Timer iconTimer, fgTimer;
    readonly string pageDir;
    readonly IconReader reader = new IconReader();
    Rectangle screen;
    IntPtr parentHwnd, insertAfter;
    string lastIcons = "";
    int lastPause = -1;
    bool manualPause, shuttingDown;

    public bool Fatal { get; private set; }
    public bool IsPrimary { get; private set; }
    public bool IsCoveredByFullscreen { get { return lastPause == 1 && !manualPause; } }

    string page;
    public Func<string> SettingsJson;
    public event Action<string> IconsChanged;
    public event Action SceneReady;
    public string LastIcons { get { return lastIcons; } }
    protected override bool ShowWithoutActivation { get { return true; } }

    public WallpaperForm(string pageDir, string page, bool manualPause, Rectangle screenBounds, bool isPrimary)
    {
        this.pageDir = pageDir;
        this.page = page;
        this.manualPause = manualPause;
        this.screen = screenBounds;
        this.IsPrimary = isPrimary;
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        BackColor = Color.FromArgb(26, 163, 196);
        Text = "DesklandWindow";

        web = new WebView2();
        web.Dock = DockStyle.Fill;
        web.DefaultBackgroundColor = BackColor;
        web.CreationProperties = new CoreWebView2CreationProperties();
        web.CreationProperties.UserDataFolder = Path.Combine(Log.DataDir, "WebView2");
        Controls.Add(web);

        iconTimer = new System.Windows.Forms.Timer();
        iconTimer.Interval = 400;
        iconTimer.Tick += delegate { PollIcons(); };
        fgTimer = new System.Windows.Forms.Timer();
        fgTimer.Interval = 500;
        fgTimer.Tick += delegate { PollForeground(); };
    }

    public void SetManualPause(bool v) { manualPause = v; PollForeground(); }

    // ---- Chụp ảnh đồng bộ màn hình khóa Windows (không vẽ biểu tượng mẫu) ----
    Action<string, string> lockSnapDone;
    System.Windows.Forms.Timer lockSnapTimeout;
    public void RequestLockSnapshot(Action<string, string> done)
    {
        if (web.CoreWebView2 == null) { done(null, I18n.T("errNotReady")); return; }
        if (lockSnapDone != null) { done(null, I18n.T("errBusyImage")); return; }
        lockSnapDone = done;
        lockSnapTimeout = new System.Windows.Forms.Timer();
        lockSnapTimeout.Interval = 15000;
        lockSnapTimeout.Tick += delegate { FinishLockSnap(null, I18n.T("errImageTimeout")); };
        lockSnapTimeout.Start();
        try { web.CoreWebView2.PostWebMessageAsString("{\"t\":\"locksnap\"}"); }
        catch (Exception ex) { FinishLockSnap(null, ex.Message); }
    }
    void OnLockSnapshot(string msg)
    {
        string[] p = msg.Split(new[] { '|' }, 3);
        if (p.Length < 2) FinishLockSnap(null, I18n.T("errBadData"));
        else if (p[1] == "error") FinishLockSnap(null, p.Length >= 3 ? p[2] : "error");
        else FinishLockSnap(p[1], null);
    }
    void FinishLockSnap(string png, string err)
    {
        if (lockSnapTimeout != null) { lockSnapTimeout.Stop(); lockSnapTimeout.Dispose(); lockSnapTimeout = null; }
        var done = lockSnapDone;
        lockSnapDone = null;
        if (done != null) done(png, err);
    }

    // ---- Chia sẻ ảnh ----
    Action<string, string, string> snapshotDone;
    System.Windows.Forms.Timer snapshotTimeout;
    public void RequestSnapshot(Action<string, string, string> done)
    {
        if (web.CoreWebView2 == null) { done(null, null, I18n.T("errNotReady")); return; }
        if (snapshotDone != null) { done(null, null, I18n.T("errBusyImage")); return; }
        snapshotDone = done;
        snapshotTimeout = new System.Windows.Forms.Timer();
        snapshotTimeout.Interval = 20000;
        snapshotTimeout.Tick += delegate { FinishSnapshot(null, null, I18n.T("errImageTimeout")); };
        snapshotTimeout.Start();
        try { web.CoreWebView2.PostWebMessageAsString("{\"t\":\"snapshot\"}"); }
        catch (InvalidOperationException ex) { FinishSnapshot(null, null, I18n.T("errNotReady")); WebDead(ex); }
        catch (COMException ex) { FinishSnapshot(null, null, I18n.T("errNotReady")); WebDead(ex); }
    }
    void OnSnapshot(string msg)
    {
        string[] p = msg.Split(new[] { '|' }, 3);
        if (p.Length < 3) FinishSnapshot(null, null, I18n.T("errBadData"));
        else if (p[1] == "error") FinishSnapshot(null, null, p[2]);
        else FinishSnapshot(p[1], p[2], null);
    }
    void FinishSnapshot(string thumb, string png, string err)
    {
        if (snapshotTimeout != null) { snapshotTimeout.Stop(); snapshotTimeout.Dispose(); snapshotTimeout = null; }
        var done = snapshotDone;
        snapshotDone = null;
        if (done != null) done(thumb, png, err);
    }

    // ---- Chia sẻ video hoạt ảnh ----
    Action<string, string, string, string> recordDone;
    System.Windows.Forms.Timer recordTimeout;
    public event Action<string> RecordProgress;
    public void RequestRecording(Action<string, string, string, string> done)
    {
        if (web.CoreWebView2 == null) { done(null, null, null, I18n.T("errNotReady")); return; }
        if (recordDone != null) { done(null, null, null, I18n.T("errBusyVideo")); return; }
        recordDone = done;
        recordTimeout = new System.Windows.Forms.Timer();
        recordTimeout.Interval = 45000;
        recordTimeout.Tick += delegate { FinishRecord(null, null, null, I18n.T("errVideoTimeout")); };
        recordTimeout.Start();
        try { web.CoreWebView2.PostWebMessageAsString("{\"t\":\"record\"}"); }
        catch (InvalidOperationException ex) { FinishRecord(null, null, null, I18n.T("errNotReady")); WebDead(ex); }
        catch (COMException ex) { FinishRecord(null, null, null, I18n.T("errNotReady")); WebDead(ex); }
    }
    void OnRecord(string msg)
    {
        string[] p = msg.Split(new[] { '|' }, 4);
        if (p.Length >= 3 && p[1] == "error") FinishRecord(null, null, null, msg.Substring(13));
        else if (p.Length < 4) FinishRecord(null, null, null, I18n.T("errBadData"));
        else FinishRecord(p[1], p[2], p[3], null);
    }
    void FinishRecord(string mime, string poster, string video, string err)
    {
        if (recordTimeout != null) { recordTimeout.Stop(); recordTimeout.Dispose(); recordTimeout = null; }
        var done = recordDone;
        recordDone = null;
        if (done != null) done(mime, poster, video, err);
    }
    public void Reload()
    {
        if (web.CoreWebView2 == null) return;
        try { web.CoreWebView2.Reload(); }
        catch (InvalidOperationException ex) { WebDead(ex); }
        catch (COMException ex) { WebDead(ex); }
    }
    public void Post(string json)
    {
        if (web.CoreWebView2 == null) return;
        try { web.CoreWebView2.PostWebMessageAsString(json); }
        catch (InvalidOperationException ex) { WebDead(ex); }
        catch (COMException ex) { WebDead(ex); }
    }

    void WebDead(Exception ex)
    {
        if (shuttingDown) return;
        Log.Write("WebView2 mất hiệu lực (runtime cập nhật hoặc tiến trình thoát), gắn lại cửa sổ hình nền: " + ex.Message);
        Shutdown();
    }
    string PageUrl { get { return "https://app.local/" + page + "?wallpaper=1"; } }

    public void LoadPage(string newPage)
    {
        page = newPage;
        if (web.CoreWebView2 == null) return;
        try { web.CoreWebView2.Navigate(PageUrl); }
        catch (InvalidOperationException ex) { WebDead(ex); }
        catch (COMException ex) { WebDead(ex); }
    }

    protected override async void OnShown(EventArgs e)
    {
        base.OnShown(e);
        try
        {
            if (!File.Exists(Path.Combine(pageDir, page)))
            {
                Fail(I18n.T("errNoPage", "path", Path.Combine(pageDir, page)));
                return;
            }
            if (!AttachToDesktop())
            {
                Fail(I18n.T("errNoDesktop"));
                return;
            }
            await web.EnsureCoreWebView2Async(null);
            if (shuttingDown || IsDisposed) return;
            var core = web.CoreWebView2;
            core.SetVirtualHostNameToFolderMapping("app.local", pageDir, CoreWebView2HostResourceAccessKind.Allow);
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.AreDevToolsEnabled = true;
            core.Settings.IsStatusBarEnabled = false;
            core.WebMessageReceived += delegate(object s, CoreWebView2WebMessageReceivedEventArgs a)
            {
                string msg = null;
                try { msg = a.TryGetWebMessageAsString(); } catch { }
                if (msg != null && msg.StartsWith("snapshot|")) { OnSnapshot(msg); return; }
                if (msg != null && msg.StartsWith("locksnap|")) { OnLockSnapshot(msg); return; }
                if (msg != null && msg.StartsWith("recprogress|")) { if (RecordProgress != null) RecordProgress(msg.Substring(12)); return; }
                if (msg != null && msg.StartsWith("record|")) { OnRecord(msg); return; }
                if (SettingsJson != null) core.PostWebMessageAsString(SettingsJson());
                lastIcons = ""; lastPause = -1; PollIcons(); PollForeground();
                if (SceneReady != null) SceneReady();
            };
            core.Navigate(PageUrl);
            iconTimer.Start();
            fgTimer.Start();
            Log.Write("WebView2 sẵn sàng: " + core.Environment.BrowserVersionString);
        }
        catch (Exception ex)
        {
            if (shuttingDown || IsDisposed) return;
            Log.Write("Khởi động thất bại: " + ex);
            Fail(I18n.T("errStart", "msg", ex.Message));
        }
    }

    void Fail(string msg)
    {
        Log.Write(msg);
        MessageBox.Show(msg + "\n\n" + I18n.T("errSeeLog", "path", Log.FilePath), "Deskland Preview");
        Fatal = true;
        Shutdown();
    }

    // ---------------------------------------------------------------- Gắn vào lớp hình nền
    public static string AttachMode = "progman";
    bool raised;
    IntPtr zAnchor;
    IntPtr winEventHook;
    Native.WinEventDelegate winEventProc;

    static IntPtr FindClassicWorker()
    {
        IntPtr host = IntPtr.Zero;
        Native.EnumWindows(delegate(IntPtr h, IntPtr l)
        {
            if (Native.FindWindowEx(h, IntPtr.Zero, "SHELLDLL_DefView", null) != IntPtr.Zero) { host = h; return false; }
            return true;
        }, IntPtr.Zero);
        return host != IntPtr.Zero ? Native.FindWindowEx(IntPtr.Zero, host, "WorkerW", null) : IntPtr.Zero;
    }

    static void RequestWorkerW(IntPtr progman, Func<bool> ready)
    {
        IntPtr res;
        Native.SendMessageTimeout(progman, 0x052C, new IntPtr(0xD), new IntPtr(1), 0, 1000, out res);
        for (int i = 0; i < 20 && !ready(); i++) Thread.Sleep(100);
    }

    bool AttachToDesktop()
    {
        if (screen.Width <= 0 || screen.Height <= 0)
            screen = Screen.PrimaryScreen.Bounds;
        IntPtr progman = Native.FindWindow("Progman", null);
        if (progman == IntPtr.Zero) { Log.Write("Không tìm thấy Progman"); return false; }
        IntPtr defView = Native.FindWindowEx(progman, IntPtr.Zero, "SHELLDLL_DefView", null);
        string mode;
        raised = defView != IntPtr.Zero;

        if (raised)
        {
            if (Native.FindWindowEx(progman, IntPtr.Zero, "WorkerW", null) == IntPtr.Zero)
                RequestWorkerW(progman, delegate { return Native.FindWindowEx(progman, IntPtr.Zero, "WorkerW", null) != IntPtr.Zero; });
            IntPtr listView = Native.FindWindowEx(defView, IntPtr.Zero, "SysListView32", null);
            if (AttachMode == "legacy")
            {
                parentHwnd = Native.FindWindowEx(progman, IntPtr.Zero, "WorkerW", null);
                if (parentHwnd == IntPtr.Zero) { Log.Write("Không tìm thấy WorkerW"); return false; }
                insertAfter = IntPtr.Zero; raised = false; mode = "24H2+ legacy: cửa sổ con WorkerW";
            }
            else if (AttachMode == "defview" && listView != IntPtr.Zero)
            {
                parentHwnd = defView; insertAfter = listView; zAnchor = listView;
                mode = "24H2+: cửa sổ con phân lớp của DefView, nằm dưới danh sách biểu tượng";
            }
            else
            {
                parentHwnd = progman; insertAfter = defView; zAnchor = defView;
                mode = "24H2+: cửa sổ con phân lớp của Progman, nằm dưới lớp biểu tượng";
            }
            if (raised)
            {
                int ex = Native.GetWindowLong(Handle, Native.GWL_EXSTYLE);
                Native.SetWindowLong(Handle, Native.GWL_EXSTYLE, ex | Native.WS_EX_LAYERED);
                Native.SetLayeredWindowAttributes(Handle, 0, 255, Native.LWA_ALPHA);
            }
        }
        else
        {
            IntPtr worker = FindClassicWorker();
            if (worker == IntPtr.Zero)
            {
                RequestWorkerW(progman, delegate { return FindClassicWorker() != IntPtr.Zero; });
                worker = FindClassicWorker();
            }
            if (worker == IntPtr.Zero) { Log.Write("Không tìm thấy WorkerW / SHELLDLL_DefView"); return false; }
            parentHwnd = worker; insertAfter = IntPtr.Zero; mode = "Classic: cửa sổ con WorkerW cấp cao nhất";
        }

        int style = Native.GetWindowLong(Handle, Native.GWL_STYLE);
        style = (style & ~(Native.WS_POPUP | Native.WS_CAPTION | Native.WS_THICKFRAME)) | Native.WS_CHILD | Native.WS_VISIBLE;
        Native.SetWindowLong(Handle, Native.GWL_STYLE, style);
        Native.SetParent(Handle, parentHwnd);

        var pt = new Native.POINT { X = screen.X, Y = screen.Y };
        Native.ScreenToClient(parentHwnd, ref pt);
        Native.SetWindowPos(Handle, insertAfter, pt.X, pt.Y, screen.Width, screen.Height,
            Native.SWP_NOACTIVATE | Native.SWP_SHOWWINDOW | Native.SWP_FRAMECHANGED);
        EnsureZOrder();

        if (raised)
        {
            uint pid;
            Native.GetWindowThreadProcessId(progman, out pid);
            winEventProc = delegate(IntPtr hook, uint ev, IntPtr hwnd, int idObject, int idChild, uint thread, uint time)
            {
                if (idObject == 0 && !shuttingDown && IsHandleCreated) EnsureZOrder();
            };
            winEventHook = Native.SetWinEventHook(0x8000 /* OBJECT_CREATE */, 0x8004 /* OBJECT_REORDER */, IntPtr.Zero,
                winEventProc, pid, 0, 0 /* WINEVENT_OUTOFCONTEXT */);
        }
        Log.Write(string.Format("Đã gắn ({0}), màn hình {1}", mode, screen));
        return true;
    }

    int zFixes;
    void EnsureZOrder()
    {
        if (!raised || zAnchor == IntPtr.Zero || !Native.IsWindow(zAnchor)) return;
        bool fixedAny = false;
        bool belowAnchor = false;
        for (IntPtr cur = Native.GetWindow(zAnchor, 2 /* GW_HWNDNEXT */); cur != IntPtr.Zero; cur = Native.GetWindow(cur, 2))
        {
            if (cur == Handle) { belowAnchor = true; break; }
        }
        if (!belowAnchor)
        {
            Native.SetWindowPos(Handle, zAnchor, 0, 0, 0, 0, Native.SWP_NOMOVE | Native.SWP_NOSIZE | Native.SWP_NOACTIVATE);
            fixedAny = true;
        }
        if (AttachMode == "progman")
        {
            for (IntPtr w = Native.FindWindowEx(parentHwnd, IntPtr.Zero, "WorkerW", null); w != IntPtr.Zero;
                 w = Native.FindWindowEx(parentHwnd, w, "WorkerW", null))
            {
                if (Native.GetWindow(w, 2) != IntPtr.Zero)
                {
                    Native.SetWindowPos(w, new IntPtr(1) /* HWND_BOTTOM */, 0, 0, 0, 0, Native.SWP_NOMOVE | Native.SWP_NOSIZE | Native.SWP_NOACTIVATE);
                    fixedAny = true;
                }
            }
        }
        if (fixedAny && ++zFixes <= 20) Log.Write("Đã chỉnh lại thứ tự lớp màn hình nền (Explorer vừa sắp xếp lại cửa sổ)");
    }

    // ---------------------------------------------------------------- Đọc biểu tượng màn hình nền
    void PollIcons()
    {
        EnsureZOrder();
        if (web.CoreWebView2 == null) return;
        bool visible;
        List<Native.RECT> rects = reader.Read(out visible);
        if (rects == null) return;
        var sb = new StringBuilder();
        sb.Append("{\"t\":\"icons\",\"vis\":").Append(visible ? "true" : "false").Append(",\"r\":[");
        int added = 0;
        for (int i = 0; i < rects.Count; i++)
        {
            Native.RECT r = rects[i];
            int cx = (r.Left + r.Right) / 2, cy = (r.Top + r.Bottom) / 2;
            if (!screen.Contains(cx, cy)) continue;
            if (added++ > 0) sb.Append(',');
            sb.Append('[').Append(r.Left - screen.X).Append(',').Append(r.Top - screen.Y).Append(',')
              .Append(r.Right - screen.X).Append(',').Append(r.Bottom - screen.Y).Append(']');
        }
        sb.Append("]}");
        string json = sb.ToString();
        if (json == lastIcons) return;
        lastIcons = json;
        if (IconsChanged != null) IconsChanged(json);
        Post(json);
        Log.Write(string.Format("Biểu tượng thay đổi (màn hình {0}): {1} mục, hiển thị={2}", screen, added, visible));
    }

    // ---------------------------------------------------------------- Tạm dừng khi có cửa sổ phóng to / toàn màn hình trên màn hình đó
    void PollForeground()
    {
        if (web.CoreWebView2 == null) return;
        bool pause = manualPause;
        string pauseBy = "";
        if (!pause)
        {
            var cls = new StringBuilder(64);
            Native.EnumWindows(delegate(IntPtr w, IntPtr l)
            {
                if (w == Handle || !Native.IsWindowVisible(w)) return true;
                int cloaked = 0;
                if (Native.DwmGetWindowAttribute(w, 14 /* DWMWA_CLOAKED */, out cloaked, 4) == 0 && cloaked != 0) return true;
                int ex = Native.GetWindowLong(w, Native.GWL_EXSTYLE);
                if ((ex & 0x00000020 /* WS_EX_TRANSPARENT */) != 0) return true;
                cls.Length = 0;
                Native.GetClassName(w, cls, 64);
                string c = cls.ToString();
                if (c == "Progman" || c == "WorkerW" || c == "Shell_TrayWnd" || c == "Shell_SecondaryTrayWnd" || c == "DummyDWMListenerWindow")
                    return true;
                Native.RECT wr;
                if (!Native.GetWindowRect(w, out wr)) return true;
                int ww = wr.Right - wr.Left, wh = wr.Bottom - wr.Top;
                if (ww <= 0 || wh <= 0) return true;
                int cx = (wr.Left + wr.Right) / 2, cy = (wr.Top + wr.Bottom) / 2;
                if (!screen.Contains(cx, cy)) return true;
                bool zoomed = Native.IsZoomed(w);
                bool covers = (ex & 0x00000080 /* WS_EX_TOOLWINDOW */) == 0 &&
                              wr.Left <= screen.Left + 2 && wr.Top <= screen.Top + 2 &&
                              wr.Right >= screen.Right - 2 && wr.Bottom >= screen.Bottom - 56;
                if (zoomed || covers)
                {
                    pause = true;
                    pauseBy = c;
                    return false;
                }
                return true;
            }, IntPtr.Zero);
        }
        int p = pause ? 1 : 0;
        if (p == lastPause) return;
        lastPause = p;
        Post("{\"t\":\"pause\",\"v\":" + (pause ? "true" : "false") + "}");
        Log.Write(string.Format("Màn hình {0} {1}", screen, pause ? "tạm dừng (có cửa sổ phóng to / toàn màn hình hoặc tạm dừng thủ công)" + (pauseBy == "" ? "" : ", lớp cửa sổ " + pauseBy) : "tiếp tục phát"));
    }

    // ---------------------------------------------------------------- Thoát: gỡ cửa sổ và để Explorer vẽ lại hình nền gốc
    public void Shutdown()
    {
        if (shuttingDown) return;
        shuttingDown = true;
        iconTimer.Stop(); fgTimer.Stop();
        reader.Dispose();
        IntPtr parent = parentHwnd;
        try { web.Dispose(); } catch { }
        if (IsHandleCreated) { Native.SetParent(Handle, IntPtr.Zero); Hide(); }
        if (parent != IntPtr.Zero && Native.IsWindow(parent))
            Native.RedrawWindow(parent, IntPtr.Zero, IntPtr.Zero, 0x1 | 0x4 | 0x80 | 0x100);
        Close();
    }

    protected override void OnHandleDestroyed(EventArgs e)
    {
        if (winEventHook != IntPtr.Zero) { Native.UnhookWinEvent(winEventHook); winEventHook = IntPtr.Zero; }
        iconTimer.Stop(); fgTimer.Stop();
        reader.Dispose();
        base.OnHandleDestroyed(e);
    }
}

// Đọc khung hình chữ nhật của các biểu tượng qua thông điệp liên tiến trình của ListView màn hình nền
class IconReader : IDisposable
{
    IntPtr listView, proc, mem;

    static IntPtr FindListView()
    {
        IntPtr defView = IntPtr.Zero;
        IntPtr progman = Native.FindWindow("Progman", null);
        if (progman != IntPtr.Zero) defView = Native.FindWindowEx(progman, IntPtr.Zero, "SHELLDLL_DefView", null);
        if (defView == IntPtr.Zero)
        {
            Native.EnumWindows(delegate(IntPtr h, IntPtr l)
            {
                IntPtr d = Native.FindWindowEx(h, IntPtr.Zero, "SHELLDLL_DefView", null);
                if (d != IntPtr.Zero) { defView = d; return false; }
                return true;
            }, IntPtr.Zero);
        }
        return defView == IntPtr.Zero ? IntPtr.Zero : Native.FindWindowEx(defView, IntPtr.Zero, "SysListView32", null);
    }

    bool Open()
    {
        Close();
        listView = FindListView();
        if (listView == IntPtr.Zero) return false;
        uint pid;
        Native.GetWindowThreadProcessId(listView, out pid);
        proc = Native.OpenProcess(0x0008 | 0x0010 | 0x0020, false, pid); // VM_OPERATION | VM_READ | VM_WRITE
        if (proc == IntPtr.Zero) return false;
        mem = Native.VirtualAllocEx(proc, IntPtr.Zero, new IntPtr(64), 0x1000 | 0x2000, 0x04);
        return mem != IntPtr.Zero;
    }

    public List<Native.RECT> Read(out bool visible)
    {
        visible = false;
        if (listView == IntPtr.Zero || mem == IntPtr.Zero || !Native.IsWindow(listView)) { if (!Open()) return null; }
        visible = Native.IsWindowVisible(listView);
        var result = new List<Native.RECT>();
        if (!visible) return result;
        int count = (int)Native.SendMessage(listView, 0x1004 /* LVM_GETITEMCOUNT */, IntPtr.Zero, IntPtr.Zero);
        var origin = new Native.POINT();
        Native.ClientToScreen(listView, ref origin);
        for (int i = 0; i < count; i++)
        {
            var r = new Native.RECT(); // Left = LVIR_BOUNDS (0)
            IntPtr n;
            if (!Native.WriteProcessMemory(proc, mem, ref r, 16, out n)) { Close(); return null; }
            Native.SendMessage(listView, 0x100E /* LVM_GETITEMRECT */, new IntPtr(i), mem);
            if (!Native.ReadProcessMemory(proc, mem, out r, 16, out n)) { Close(); return null; }
            r.Left += origin.X; r.Right += origin.X; r.Top += origin.Y; r.Bottom += origin.Y;
            result.Add(r);
        }
        return result;
    }

    void Close()
    {
        if (mem != IntPtr.Zero && proc != IntPtr.Zero) Native.VirtualFreeEx(proc, mem, IntPtr.Zero, 0x8000);
        if (proc != IntPtr.Zero) Native.CloseHandle(proc);
        mem = proc = IntPtr.Zero;
    }

    public void Dispose() { Close(); }
}

static class Native
{
    public const int GWL_STYLE = -16, GWL_EXSTYLE = -20, WS_EX_LAYERED = 0x00080000;
    public const uint LWA_ALPHA = 0x2, SWP_NOSIZE = 0x0001, SWP_NOMOVE = 0x0002;
    public const uint MOD_ALT = 0x1, MOD_CONTROL = 0x2, MOD_NOREPEAT = 0x4000;
    public const int SW_SHOW = 5, SW_RESTORE = 9;
    [DllImport("user32.dll")] public static extern bool RegisterHotKey(IntPtr h, int id, uint modifiers, uint vk);
    [DllImport("dwmapi.dll")] public static extern int DwmSetWindowAttribute(IntPtr h, int attr, ref int val, int size);
    [DllImport("user32.dll")] public static extern bool UnregisterHotKey(IntPtr h, int id);
    [DllImport("user32.dll")] public static extern bool AllowSetForegroundWindow(int pid);
    [DllImport("user32.dll")] public static extern uint GetDpiForSystem();
    [DllImport("wtsapi32.dll", CharSet = CharSet.Unicode)] public static extern bool WTSQuerySessionInformation(IntPtr server, int session, int infoClass, out IntPtr buffer, out int bytes);
    [DllImport("wtsapi32.dll")] public static extern void WTSFreeMemory(IntPtr p);
    public delegate void WinEventDelegate(IntPtr hook, uint ev, IntPtr hwnd, int idObject, int idChild, uint thread, uint time);
    [DllImport("user32.dll")] public static extern bool SetLayeredWindowAttributes(IntPtr h, uint key, byte alpha, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
    [DllImport("user32.dll")] public static extern IntPtr SetWinEventHook(uint min, uint max, IntPtr mod, WinEventDelegate proc, uint pid, uint tid, uint flags);
    [DllImport("user32.dll")] public static extern bool UnhookWinEvent(IntPtr hook);
    public const int WS_CHILD = 0x40000000, WS_VISIBLE = 0x10000000, WS_POPUP = unchecked((int)0x80000000);
    public const int WS_CAPTION = 0x00C00000, WS_THICKFRAME = 0x00040000;
    public const uint SWP_NOACTIVATE = 0x0010, SWP_SHOWWINDOW = 0x0040, SWP_FRAMECHANGED = 0x0020;

    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
    [StructLayout(LayoutKind.Sequential)] public struct KBDLLHOOKSTRUCT { public uint vkCode, scanCode, flags, time; public IntPtr dwExtraInfo; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    public delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll")] public static extern IntPtr SetWindowsHookEx(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);
    [DllImport("user32.dll")] public static extern bool UnhookWindowsHookEx(IntPtr hhk);
    [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr GetModuleHandle(string lpModuleName);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vKey);
    [DllImport("user32.dll")] public static extern bool LockWorkStation();
    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern bool LogonUser(string lpszUsername, string lpszDomain, string lpszPassword, int dwLogonType, int dwLogonProvider, out IntPtr phToken);

    public static bool VerifyWindowsPassword(string password, bool allowBlankIfAccountHasNoPassword)
    {
        if (password == null) return false;
        string user = Environment.UserName;
        string domain = Environment.UserDomainName;
        string[] domains = new[] { ".", domain, null };
        int[] logonTypes = new[] { 2 /* LOGON32_LOGON_INTERACTIVE */, 3 /* LOGON32_LOGON_NETWORK */ };
        if (password.Length > 0)
        {
            for (int d = 0; d < domains.Length; d++)
            {
                for (int t = 0; t < logonTypes.Length; t++)
                {
                    IntPtr token = IntPtr.Zero;
                    try
                    {
                        if (LogonUser(user, domains[d], password, logonTypes[t], 0, out token))
                            return true;
                    }
                    catch { }
                    finally { if (token != IntPtr.Zero) CloseHandle(token); }
                }
            }
        }
        if (allowBlankIfAccountHasNoPassword)
        {
            IntPtr emptyTok = IntPtr.Zero;
            try
            {
                if (LogonUser(user, ".", "", 2, 0, out emptyTok) || Marshal.GetLastWin32Error() == 1327 /* ERROR_ACCOUNT_RESTRICTION */)
                    return true;
            }
            catch { }
            finally { if (emptyTok != IntPtr.Zero) CloseHandle(emptyTok); }
        }
        return false;
    }

    [DllImport("user32.dll")] public static extern bool GetLastInputInfo(ref LASTINPUTINFO plii);
    public static uint GetIdleMilliseconds()
    {
        var lii = new LASTINPUTINFO { cbSize = (uint)Marshal.SizeOf(typeof(LASTINPUTINFO)) };
        if (!GetLastInputInfo(ref lii)) return 0;
        return unchecked((uint)Environment.TickCount - lii.dwTime);
    }

    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr h, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern IntPtr SetParent(IntPtr child, IntPtr parent);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int index);
    [DllImport("user32.dll")] public static extern int SetWindowLong(IntPtr h, int index, int value);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr h, IntPtr rect, IntPtr rgn, uint flags);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attribute, out int value, int size);
    [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll")] public static extern IntPtr VirtualAllocEx(IntPtr proc, IntPtr addr, IntPtr size, uint type, uint protect);
    [DllImport("kernel32.dll")] public static extern bool VirtualFreeEx(IntPtr proc, IntPtr addr, IntPtr size, uint type);
    [DllImport("kernel32.dll")] public static extern bool WriteProcessMemory(IntPtr proc, IntPtr addr, ref RECT buf, int size, out IntPtr written);
    [DllImport("kernel32.dll")] public static extern bool ReadProcessMemory(IntPtr proc, IntPtr addr, out RECT buf, int size, out IntPtr read);
}

// Deskland Preview has its own data and instance identity; isolated profiles support installation QA.
static class InstallProfile
{
    public static readonly string Override = Environment.GetEnvironmentVariable("DESKLAND_PROFILE_DIR");
    public static readonly bool Isolated = !string.IsNullOrEmpty(Override);
    public static readonly string DataDir = Isolated ? Path.GetFullPath(Override) : Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "DesklandPreview");
    public static readonly string Suffix = MakeSuffix();
    static string MakeSuffix()
    {
        if (!Isolated) return "_Preview";
        using (var hash = System.Security.Cryptography.SHA256.Create())
            return "_Preview_" + BitConverter.ToString(hash.ComputeHash(Encoding.UTF8.GetBytes(DataDir.ToUpperInvariant()))).Replace("-", "").Substring(0, 24);
    }

    // An installer may only close the instance using this exact executable path.
    public static int StopInstalled()
    {
        foreach (Process process in Process.GetProcessesByName("Deskland"))
        {
            using (process)
            {
                if (process.Id == Process.GetCurrentProcess().Id) continue;
                try
                {
                    if (!string.Equals(process.MainModule.FileName, Application.ExecutablePath, StringComparison.OrdinalIgnoreCase)) continue;
                    EventWaitHandle ev;
                    if (!EventWaitHandle.TryOpenExisting(Program.ExitEventName, out ev)) return 1;
                    using (ev) ev.Set();
                    if (!process.WaitForExit(10000)) return 1;
                }
                catch (InvalidOperationException) { }
                catch { return 1; }
            }
        }
        return 0;
    }
}
