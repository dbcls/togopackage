#!/usr/bin/env ruby
# frozen_string_literal: true

require 'yaml'
require 'date'
require 'fileutils'
require 'optparse'

require_relative 'rdf_config_query_generator'

def parse_options
  opts = {
    rdf_config_base_dir: '/data/rdf-config',
    output_dir: nil,
    sparql_endpoint: 'http://localhost:7002/sparql'
  }

  OptionParser.new do |parser|
    parser.on('--rdf-config-base-dir DIR') { |v| opts[:rdf_config_base_dir] = v }
    parser.on('--output-dir DIR') { |v| opts[:output_dir] = v }
    parser.on('--sparql-endpoint URL') { |v| opts[:sparql_endpoint] = v }
  end.parse!

  if opts[:output_dir].to_s.strip.empty?
    warn 'Usage: generate_sparqlist_from_rdf_config.rb --output-dir DIR [--rdf-config-base-dir DIR]'
    exit(1)
  end

  opts
end

def load_yaml(path)
  YAML.load_file(path, permitted_classes: [Date, Time, Symbol])
rescue StandardError => e
  raise "Failed to load YAML: #{path} (#{e.message})"
end

def sanitize_filename(name)
  value = name.to_s.downcase.gsub(/[^a-z0-9_-]+/, '_').gsub(/\A_+|_+\z/, '')
  value.empty? ? 'query' : value
end

def pick_endpoint(override_endpoint)
  endpoint = override_endpoint.to_s.strip
  raise 'SPARQL endpoint is empty' if endpoint.empty?

  endpoint
end

def normalize_parameters(query_config)
  return {} unless query_config.is_a?(Hash)

  parameters = query_config['parameters']
  case parameters
  when Hash
    parameters
  when Array
    parameters.to_h { |name| [name.to_s, ''] }
  else
    {}
  end
end

def generate_sparql(config_dir, query_name, parameter_names)
  RDFConfigQueryGenerator.generate_named_query(
    config_dir,
    query_name,
    parameter_names: parameter_names
  )
end

def render_markdown(config_name:, query_name:, description:, parameters:, endpoint:, query:)
  lines = []
  lines << "# #{config_name} / #{query_name}"
  lines << ''

  unless description.empty?
    lines << description
    lines << ''
  end

  unless parameters.empty?
    lines << '## Parameters'
    lines << ''
    parameters.each do |name, default_value|
      lines << "* `#{name}`"
      default_text = Array(default_value).map(&:to_s).join(', ')
      lines << "  * default: #{default_text}" unless default_text.empty?
    end
    lines << ''
  end

  lines << '## Endpoint'
  lines << ''
  lines << endpoint
  lines << ''

  lines << "## `results` #{query_name}"
  lines << ''
  lines << '```sparql'
  lines << query.rstrip
  lines << '```'
  lines << ''

  lines.join("\n")
end

def write_sparqlets(opts)
  base_dir = File.expand_path(opts[:rdf_config_base_dir])
  output_dir = File.expand_path(opts[:output_dir])
  endpoint = pick_endpoint(opts[:sparql_endpoint])

  raise "RDF-config base directory does not exist: #{base_dir}" unless Dir.exist?(base_dir)

  FileUtils.mkdir_p(output_dir)
  count = 0
  used_names = {}

  Dir.children(base_dir).sort.each do |entry|
    config_dir = File.join(base_dir, entry)
    next unless File.directory?(config_dir)
    next unless File.exist?(File.join(config_dir, 'model.yaml'))
    next unless File.exist?(File.join(config_dir, 'sparql.yaml'))

    sparql_config = load_yaml(File.join(config_dir, 'sparql.yaml'))
    raise "Invalid sparql.yaml format: #{config_dir}" unless sparql_config.is_a?(Hash)

    sparql_config.each do |query_name, query_config|
      next unless query_name

      parameters = normalize_parameters(query_config)
      generated_query = generate_sparql(config_dir, query_name.to_s, parameters.keys)

      filename_base = "#{sanitize_filename(entry)}__#{sanitize_filename(query_name)}"
      filename = "#{filename_base}.md"
      if used_names.key?(filename)
        used_names[filename] += 1
        filename = "#{filename_base}_#{used_names[filename]}.md"
      else
        used_names[filename] = 0
      end

      markdown = render_markdown(
        config_name: entry,
        query_name: query_name.to_s,
        description: query_config.is_a?(Hash) ? query_config.fetch('description', '').to_s.strip : '',
        parameters: parameters,
        endpoint: endpoint,
        query: generated_query
      )

      File.write(File.join(output_dir, filename), markdown)
      count += 1
    end
  end

  if count.zero?
    warn 'No SPARQList files generated from RDF-config.'
    exit 10
  end

  count
end

options = parse_options
count = write_sparqlets(options)
puts "Generated #{count} SPARQList files."
