((sql-ts-mode
  . ((eglot-workspace-configuration
      . (:sqls
         (:connections
          [(:alias "project"
            :driver "sqlite3"
            :dataSourceName "./build.db")]))))))
