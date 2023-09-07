# Downloads a file from a URL.  The URL may be https, and things will still work.
# URL - the URL to dowload the file from
# DESTFILE - the location to save the download to.  The file there will be overwritten if it exists.
# FAIL_ON_ERROR - whether or not to print a FATAL_ERROR if the download fails.
function(download_windows_file_https URL DESTFILE FAIL_ON_ERROR)

  #Fixe issue with PROXY on windows to avoid env variable HTTP_PROXY with pwd
  find_program(POWERSHELL NAMES powershell DOC "Path to the Windows Powershell executable.  Used to download files")
  set(HAVE_DOWNLOADER_PROGRAM TRUE)

  set(DOWNLOADER_PROGRAM ${POWERSHELL})
    
  # from http://superuser.com/questions/25538/how-to-download-files-from-command-line-in-windows-like-wget-is-doing

  set(DOWNLOAD_COMMAND "")
    if(NOT HAVE_DOWNLOADER_PROGRAM)
        message(FATAL_ERROR "A downloader program, either curl, wget, or powershell, is required to download files.  Please set CURL, WGET, or POWERSHELL to the location of the respective program.")
    endif()

  SET(SCRIPT_DOWNLOAD "${CMAKE_SOURCE_DIR}/cmake/downloader.ps1")

  cmake_path(NATIVE_PATH SCRIPT_DOWNLOAD script_download)
  cmake_path(NATIVE_PATH DESTFILE dest_win_file)
  #${URL} ${dest_win_file}
  SET(CONFIGURED_DOWNLOAD_COMMAND "${script_download}")

    #message("Executing command: ${DOWNLOADER_PROGRAM} ${CONFIGURED_DOWNLOAD_COMMAND}")
    
    execute_process(COMMAND "${DOWNLOADER_PROGRAM}" -ExecutionPolicy Bypass -File "${CONFIGURED_DOWNLOAD_COMMAND}" ${URL} ${dest_win_file} RESULT_VARIABLE DOWNLOAD_RESULT OUTPUT_VARIABLE out_var ERROR_VARIABLE err_var)


    if((NOT ${DOWNLOAD_RESULT} EQUAL 0 OR NOT "${err_var}" STREQUAL "") AND FAIL_ON_ERROR)
        message(STATUS "Unable to download file ${URL} msg ${DOWNLOAD_RESULT} : ${out_var} error : ${err_var}")
    message(FATAL_ERROR "${DOWNLOADER_PROGRAM} ${CONFIGURED_DOWNLOAD_COMMAND}")
  else()
    message(STATUS "Info sucess to download file ${URL} msg ${DOWNLOAD_RESULT} : [${out_var}] error : [${err_var}]")
    endif()
endfunction(download_windows_file_https URL DESTFILE FAIL_ON_ERROR)
