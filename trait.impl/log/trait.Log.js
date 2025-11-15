(function() {
    var implementors = Object.fromEntries([["log",[]],["sel4_logging",[["impl <a class=\"trait\" href=\"log/trait.Log.html\" title=\"trait log::Log\">Log</a> for <a class=\"struct\" href=\"sel4_logging/struct.Logger.html\" title=\"struct sel4_logging::Logger\">Logger</a>"],["impl&lt;R: <a class=\"trait\" href=\"lock_api/mutex/trait.RawMutex.html\" title=\"trait lock_api::mutex::RawMutex\">RawMutex</a> + Send + Sync, T: <a class=\"trait\" href=\"log/trait.Log.html\" title=\"trait log::Log\">Log</a>&gt; <a class=\"trait\" href=\"log/trait.Log.html\" title=\"trait log::Log\">Log</a> for <a class=\"struct\" href=\"sel4_logging/struct.SynchronizedLogger.html\" title=\"struct sel4_logging::SynchronizedLogger\">SynchronizedLogger</a>&lt;R, T&gt;"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[10,699]}